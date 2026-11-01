#!/usr/bin/env python3
"""Install a versioned Inodune build under a user-writable absolute prefix."""
from __future__ import annotations

import argparse
import json
import os
import re
import stat
import sys
import tarfile
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
PORTABLE_BUNDLE = (SCRIPT_DIR / "VERSION").is_file()
ROOT = SCRIPT_DIR if PORTABLE_BUNDLE else SCRIPT_DIR.parent
SOURCES = (
    {
        "inodune-desktop": ROOT / "bin" / "inodune-desktop",
        "inodune-cli": ROOT / "bin" / "inodune-cli",
    }
    if PORTABLE_BUNDLE
    else {
        "inodune-desktop": ROOT / "target" / "compat" / "release" / "inodune-desktop",
        "inodune-cli": ROOT / "target" / "compat" / "release" / "inodune-cli",
    }
)


def safe_component(path: Path, *, create: bool) -> None:
    try:
        info = path.lstat()
    except FileNotFoundError:
        if not create:
            raise
        path.mkdir(mode=0o755)
        info = path.lstat()
    if not stat.S_ISDIR(info.st_mode):
        raise RuntimeError(f"Refusing non-directory or symlink path: {path}")
    if info.st_uid not in (0, os.geteuid()):
        raise RuntimeError(f"Refusing directory owned by another UID: {path}")
    if info.st_mode & 0o022 and not (info.st_uid == 0 and info.st_mode & stat.S_ISVTX):
        raise RuntimeError(f"Refusing group/world-writable directory: {path}")


def ensure_directory(path: Path) -> None:
    if not path.is_absolute():
        raise RuntimeError(f"Expected an absolute path: {path}")
    current = Path(path.anchor)
    safe_component(current, create=False)
    for name in path.parts[1:]:
        current = current / name
        safe_component(current, create=True)


def read_source(path: Path, executable: bool) -> bytes:
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, "rb") as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid():
            raise RuntimeError(f"Refusing unexpected build artifact: {path}")
        if executable and not (info.st_mode & 0o111):
            raise RuntimeError(f"Build artifact is not executable: {path}")
        return source.read()


def desktop_escape(value: str) -> str:
    quoted = value.replace("%", "%%").replace("\\", "\\\\").replace("\"", "\\\"").replace("`", "\\`").replace("$", "\\$")
    # Desktop string escapes are interpreted before Exec argument quoting.
    return '"' + desktop_string(quoted) + '"'


def desktop_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r")


def create_exclusive(path: Path, contents: bytes, mode: int) -> None:
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    with os.fdopen(fd, "wb") as output:
        output.write(contents)
        output.flush()
        os.fsync(output.fileno())


def archive_payload(path: Path) -> tuple[str, dict[str, bytes]]:
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_size > 64 * 1024 * 1024:
        raise RuntimeError(f"Refusing unexpected archive file: {path}")
    limits = {
        "VERSION": 64,
        "LICENSE": 1024 * 1024,
        "inodune-icon-512.png": 1024 * 1024,
        "bin/inodune-desktop": 32 * 1024 * 1024,
        "bin/inodune-cli": 16 * 1024 * 1024,
        "install-user.py": 256 * 1024,
    }
    with tarfile.open(path, mode="r|gz") as archive:
        content: dict[str, bytes] = {}
        for member in archive:
            name = member.name
            if name not in limits or name in content:
                raise RuntimeError("Archive has unexpected or duplicate entries")
            limit = limits[name]
            if not member.isfile() or member.size < 0 or member.size > limit:
                raise RuntimeError(f"Archive entry is not a bounded regular file: {name}")
            source = archive.extractfile(member)
            if source is None:
                raise RuntimeError(f"Archive entry cannot be read: {name}")
            data = source.read(limit + 1)
            if len(data) != member.size or len(data) > limit:
                raise RuntimeError(f"Archive entry size does not match: {name}")
            content[name] = data
        if set(content) != set(limits):
            raise RuntimeError("Archive is missing required entries")
    version = content["VERSION"].decode("utf-8").strip()
    for name in ("inodune-desktop", "inodune-cli"):
        elf = content[f"bin/{name}"]
        if len(elf) < 20 or elf[:4] != b"\x7fELF" or int.from_bytes(elf[18:20], "little") != 62:
            raise RuntimeError(f"Archive executable is not x86_64 ELF: {name}")
    if not content["inodune-icon-512.png"].startswith(b"\x89PNG\r\n\x1a\n"):
        raise RuntimeError("Archive icon is not a PNG")
    return version, content


def install(prefix: Path, archive: Path | None = None) -> Path:
    prefix = prefix.expanduser()
    if not prefix.is_absolute():
        raise RuntimeError("--prefix must be an absolute path")
    if ".." in prefix.parts:
        raise RuntimeError("--prefix cannot contain parent-directory components")
    if any(ord(c) < 32 or ord(c) == 127 for c in str(prefix)) or "=" in str(prefix):
        raise RuntimeError("--prefix contains an unsupported control character")

    if archive is not None:
        version, archived = archive_payload(archive.expanduser())
        file_data = {
            "inodune-desktop": archived["bin/inodune-desktop"],
            "inodune-cli": archived["bin/inodune-cli"],
            "LICENSE": archived["LICENSE"],
            "inodune-icon-512.png": archived["inodune-icon-512.png"],
        }
    else:
        file_data = {}
        config_path = ROOT / "apps" / "desktop" / "src-tauri" / "tauri.conf.json"
        if PORTABLE_BUNDLE:
            version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
        elif config_path.is_file():
            version = json.loads(config_path.read_text(encoding="utf-8"))["version"]
        else:
            raise RuntimeError("--archive is required when install-user.py is used outside the source tree")
        for name, source in SOURCES.items():
            file_data[name] = read_source(source, executable=True)
        license_file = ROOT / "LICENSE"
        icon_file = ROOT / "inodune-icon-512.png" if PORTABLE_BUNDLE else ROOT / "assets" / "brand" / "inodune-icon-512.png"
        file_data["LICENSE"] = read_source(license_file, executable=False)
        file_data["inodune-icon-512.png"] = read_source(icon_file, executable=False)
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise RuntimeError(f"Unsupported version string for a user install: {version!r}")
    appdir = prefix / "lib" / "inodune" / version
    launcher_dir = prefix / "share" / "applications"
    launcher = launcher_dir / f"inodune-{version}.desktop"

    # Detect every final collision before creating any path.
    for path in (appdir, launcher):
        if path.exists() or path.is_symlink():
            raise FileExistsError(f"Refusing to overwrite existing path: {path}")

    if prefix.exists():
        info = prefix.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid():
            raise RuntimeError(f"--prefix must be an existing directory owned by this user: {prefix}")
    ensure_directory(prefix)
    ensure_directory(appdir.parent)
    ensure_directory(launcher_dir)
    # mkdir is exclusive: a racing or pre-existing version cannot be replaced.
    appdir.mkdir(mode=0o755)
    binary_dir = appdir / "bin"
    binary_dir.mkdir(mode=0o755)
    for name in SOURCES:
        destination = binary_dir / name
        create_exclusive(destination, file_data[name], 0o755)
    create_exclusive(appdir / "LICENSE", file_data["LICENSE"], 0o644)
    icon_path = appdir / "inodune-icon-512.png"
    create_exclusive(icon_path, file_data["inodune-icon-512.png"], 0o644)
    desktop = (
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=Inodune\n"
        "Comment=Free up space. Preserve what matters.\n"
        f"Exec={desktop_escape(str(binary_dir / 'inodune-desktop'))}\n"
        f"TryExec={desktop_string(str(binary_dir / 'inodune-desktop'))}\n"
        f"Icon={desktop_string(str(icon_path))}\n"
        "Terminal=false\n"
        "Categories=Utility;\n"
        "StartupWMClass=app.inodune.desktop\n"
    ).encode("utf-8")
    create_exclusive(launcher, desktop, 0o644)
    return appdir


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--prefix",
        type=Path,
        default=Path.home() / ".local",
        help="absolute user-owned install prefix (default: ~/.local)",
    )
    parser.add_argument("--archive", type=Path, help="read binaries and assets from a generated Inodune tar.gz")
    if PORTABLE_BUNDLE and "--archive" in sys.argv:
        parser.error("use either an extracted portable bundle or --archive")
    args = parser.parse_args()
    try:
        location = install(args.prefix, args.archive)
    except (FileExistsError, OSError, RuntimeError, KeyError, ValueError) as error:
        print(f"Inodune user install refused or incomplete: {error}", file=sys.stderr)
        return 1
    print(f"Installed Inodune under {location}")
    print(f"CLI: {location / 'bin' / 'inodune-cli'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
