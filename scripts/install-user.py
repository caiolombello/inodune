#!/usr/bin/env python3
"""Install a versioned Inodune build under a user-writable absolute prefix."""
from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import re
import secrets
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
MANIFEST_NAME = "INSTALL-MANIFEST.json"
UNINSTALL_JOURNAL_PREFIX = ".inodune-uninstall-"
MANIFEST_LIMIT = 64 * 1024
APP_FILES = {
    "bin/inodune-desktop": 0o755,
    "bin/inodune-cli": 0o755,
    "LICENSE": 0o644,
    "inodune-icon-512.png": 0o644,
}


class InstallRefused(RuntimeError):
    """The install tree contains a file the installer cannot safely own."""


class LifecycleReport:
    def __init__(self, *, applied: bool, complete: bool = True, lines: list[str]):
        self.applied = applied
        self.complete = complete
        self.lines = lines


def require_unprivileged_user() -> None:
    if os.geteuid() == 0:
        raise InstallRefused("The per-user installer must be run as the desktop user, without root")


def version_key(value: str) -> tuple[int, int, int]:
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", value):
        raise InstallRefused(f"Unsupported version string: {value!r}")
    return tuple(int(part) for part in value.split("."))


def checked_directory(path: Path) -> os.stat_result:
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid not in (0, os.geteuid()):
        raise InstallRefused(f"Refusing non-directory, link, or foreign directory: {path}")
    if info.st_mode & 0o022 and not (info.st_uid == 0 and info.st_mode & stat.S_ISVTX):
        raise InstallRefused(f"Refusing group/world-writable directory: {path}")
    return info


def checked_install_directory(path: Path) -> os.stat_result:
    info = checked_directory(path)
    if info.st_uid != os.geteuid():
        raise InstallRefused(f"Refusing application directory owned by another UID: {path}")
    return info


def check_existing_parents(path: Path) -> None:
    if not path.is_absolute() or ".." in path.parts:
        raise InstallRefused("Expected a normalized absolute path")
    current = Path(path.anchor)
    checked_directory(current)
    for name in path.parts[1:]:
        current = current / name
        checked_directory(current)


def read_regular(path: Path, *, mode: int | None = None, limit: int = MANIFEST_LIMIT) -> tuple[bytes, os.stat_result]:
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_nlink != 1:
            raise InstallRefused(f"Refusing non-private, linked, or unexpected file: {path}")
        if mode is not None and stat.S_IMODE(info.st_mode) != mode:
            raise InstallRefused(f"Refusing file with changed permissions: {path}")
        if info.st_size > limit:
            raise InstallRefused(f"Refusing oversized file: {path}")
        data = source.read(limit + 1)
        if len(data) != info.st_size or len(data) > limit:
            raise InstallRefused(f"File changed while being inspected: {path}")
        return data, info


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
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, mode)
    with os.fdopen(fd, "wb") as output:
        os.fchmod(output.fileno(), mode)
        output.write(contents)
        output.flush()
        os.fsync(output.fileno())


def create_exclusive_at(
    dir_fd: int,
    name: str,
    relative: str,
    contents: bytes,
    mode: int,
    created: list[tuple[str, int, int]],
) -> None:
    fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, mode, dir_fd=dir_fd)
    try:
        os.fchmod(fd, mode)
        info = os.fstat(fd)
        created.append((relative, info.st_dev, info.st_ino))
        view = memoryview(contents)
        while view:
            written = os.write(fd, view)
            if written <= 0:
                raise OSError("staging write made no progress")
            view = view[written:]
        os.fsync(fd)
    finally:
        os.close(fd)


def rename_noreplace_at(dir_fd: int, source: str, destination: str) -> None:
    try:
        libc = ctypes.CDLL(None, use_errno=True)
        renameat2 = libc.renameat2
    except AttributeError as error:
        raise InstallRefused("This Linux libc does not expose renameat2; installation refused safely") from error
    renameat2.argtypes = (ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint)
    renameat2.restype = ctypes.c_int
    if renameat2(dir_fd, os.fsencode(source), dir_fd, os.fsencode(destination), 1) != 0:
        error_number = ctypes.get_errno()
        raise OSError(error_number, os.strerror(error_number), destination)


def cleanup_staging_dir(
    app_root_fd: int,
    stage_name: str,
    stage_identity: tuple[int, int],
    bin_identity: tuple[int, int] | None,
    created: list[tuple[str, int, int]],
) -> None:
    stage_fd = os.open(stage_name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=app_root_fd)
    try:
        stage_info = os.fstat(stage_fd)
        if (stage_info.st_dev, stage_info.st_ino) != stage_identity or stage_info.st_uid != os.geteuid():
            raise InstallRefused(f"Staging directory changed; inspect it manually: {stage_name}")
        for relative, device, inode in reversed(created):
            parent_fd = stage_fd
            leaf = relative
            child_fd = None
            if "/" in relative:
                parent, leaf = relative.split("/", 1)
                if parent != "bin":
                    raise InstallRefused(f"Unexpected staged path; inspect it manually: {relative}")
                child_fd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=stage_fd)
                parent_fd = child_fd
            try:
                try:
                    current = os.stat(leaf, dir_fd=parent_fd, follow_symlinks=False)
                except FileNotFoundError:
                    continue
                if (current.st_dev, current.st_ino) != (device, inode) or not stat.S_ISREG(current.st_mode):
                    raise InstallRefused(f"Staged file changed; preserved for inspection: {relative}")
                os.unlink(leaf, dir_fd=parent_fd)
            finally:
                if child_fd is not None:
                    os.close(child_fd)
        if bin_identity is not None:
            current_bin = os.stat("bin", dir_fd=stage_fd, follow_symlinks=False)
            if (current_bin.st_dev, current_bin.st_ino) != bin_identity or not stat.S_ISDIR(current_bin.st_mode):
                raise InstallRefused("Staging bin directory changed; preserved for inspection")
            os.rmdir("bin", dir_fd=stage_fd)
    finally:
        os.close(stage_fd)
    current_stage = os.stat(stage_name, dir_fd=app_root_fd, follow_symlinks=False)
    if (current_stage.st_dev, current_stage.st_ino) != stage_identity or not stat.S_ISDIR(current_stage.st_mode):
        raise InstallRefused(f"Staging directory changed; inspect it manually: {stage_name}")
    os.rmdir(stage_name, dir_fd=app_root_fd)


def launcher_path(prefix: Path) -> Path:
    return prefix / "share" / "applications" / "inodune.desktop"


def launcher_bytes(prefix: Path, version: str) -> bytes:
    binary_dir = prefix / "lib" / "inodune" / version / "bin"
    icon_path = prefix / "lib" / "inodune" / version / "inodune-icon-512.png"
    return (
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


def create_manifest(prefix: Path, appdir: Path, version: str) -> bytes:
    files = {}
    for name, mode in APP_FILES.items():
        data, info = read_regular(appdir / name, mode=mode, limit=64 * 1024 * 1024)
        files[name] = {"sha256": hashlib.sha256(data).hexdigest(), "size": info.st_size, "mode": mode}
    record = {
        "schema": 1,
        "version": version,
        "prefix": str(prefix),
        "files": files,
        "launcher_sha256": hashlib.sha256(launcher_bytes(prefix, version)).hexdigest(),
    }
    return (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()


def uninstall_journal_path(prefix: Path, version: str) -> Path:
    return prefix / "lib" / "inodune" / f"{UNINSTALL_JOURNAL_PREFIX}{version}.json"


def validate_manifest_record(prefix: Path, version: str, manifest: object) -> dict[str, object]:
    if not isinstance(manifest, dict) or set(manifest) != {"schema", "version", "prefix", "files", "launcher_sha256"}:
        raise InstallRefused("Install manifest schema is unknown")
    if manifest["schema"] != 1 or manifest["version"] != version or manifest["prefix"] != str(prefix):
        raise InstallRefused("Install manifest does not match this version and prefix")
    file_records = manifest["files"]
    if not isinstance(file_records, dict) or set(file_records) != set(APP_FILES):
        raise InstallRefused("Install manifest file list is incomplete or contains unexpected paths")
    if not isinstance(manifest["launcher_sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", manifest["launcher_sha256"]):
        raise InstallRefused("Install manifest launcher checksum is invalid")
    for name, mode in APP_FILES.items():
        record = file_records[name]
        if not isinstance(record, dict) or set(record) != {"sha256", "size", "mode"}:
            raise InstallRefused(f"Install manifest entry is malformed: {name}")
        if record["mode"] != mode or type(record["size"]) is not int or not 0 <= record["size"] <= 64 * 1024 * 1024:
            raise InstallRefused(f"Install manifest entry is invalid: {name}")
        if not isinstance(record["sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", record["sha256"]):
            raise InstallRefused(f"Install manifest digest is invalid: {name}")
    return manifest


def read_uninstall_journal(prefix: Path, version: str) -> dict[str, object] | None:
    journal_path = uninstall_journal_path(prefix, version)
    try:
        raw, _ = read_regular(journal_path, mode=0o600)
    except FileNotFoundError:
        return None
    try:
        journal = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise InstallRefused(f"Uninstall journal is unreadable: {error}") from error
    if not isinstance(journal, dict) or set(journal) != {"schema", "version", "prefix", "manifest"}:
        raise InstallRefused("Uninstall journal schema is unknown")
    if journal["schema"] != 1 or journal["version"] != version or journal["prefix"] != str(prefix):
        raise InstallRefused("Uninstall journal does not match this version and prefix")
    return validate_manifest_record(prefix, version, journal["manifest"])


def read_manifest(
    prefix: Path,
    version: str,
    *,
    appdir_override: Path | None = None,
) -> tuple[Path, dict[str, object]]:
    version_key(version)
    appdir = appdir_override or prefix / "lib" / "inodune" / version
    journal = None if appdir_override is not None else read_uninstall_journal(prefix, version)
    try:
        check_existing_parents(appdir)
    except FileNotFoundError:
        if journal is not None:
            try:
                appdir.lstat()
            except FileNotFoundError:
                return appdir, journal
        raise
    checked_install_directory(prefix)
    checked_install_directory(appdir)
    if (appdir / "bin").exists() or (appdir / "bin").is_symlink():
        checked_install_directory(appdir / "bin")
    try:
        raw, _ = read_regular(appdir / MANIFEST_NAME, mode=0o600)
        try:
            disk_manifest = json.loads(raw)
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise InstallRefused(f"Install manifest is unreadable: {error}") from error
        manifest = validate_manifest_record(prefix, version, disk_manifest)
        if journal is not None and manifest != journal:
            raise InstallRefused("Uninstall journal does not match the install manifest")
    except FileNotFoundError:
        if journal is None:
            raise
        manifest = journal
    file_records = manifest["files"]

    # Inspect the complete small install tree. Never recurse through a link,
    # and never remove a file not explicitly named by the install manifest.
    expected_tree = {MANIFEST_NAME, "bin", "LICENSE", "inodune-icon-512.png", *APP_FILES}
    observed_tree: set[str] = set()
    with os.scandir(appdir) as entries:
        for entry in entries:
            observed_tree.add(entry.name)
            if entry.name not in {MANIFEST_NAME, "bin", "LICENSE", "inodune-icon-512.png"} or entry.is_symlink():
                raise InstallRefused(f"Unexpected file or link in application directory: {entry.path}")
    try:
        with os.scandir(appdir / "bin") as entries:
            for entry in entries:
                observed_tree.add(f"bin/{entry.name}")
                if entry.name not in {"inodune-desktop", "inodune-cli"} or entry.is_symlink():
                    raise InstallRefused(f"Unexpected file or link in application directory: {entry.path}")
    except FileNotFoundError:
        if journal is None:
            raise
    if journal is None and observed_tree != expected_tree:
        raise InstallRefused("Installed application tree differs from the manifest")
    for name, mode in APP_FILES.items():
        record = file_records[name]
        try:
            data, info = read_regular(appdir / name, mode=mode, limit=64 * 1024 * 1024)
        except FileNotFoundError:
            if journal is not None:
                continue
            raise
        if info.st_size != record["size"] or hashlib.sha256(data).hexdigest() != record["sha256"]:
            raise InstallRefused(f"Installed file changed since installation: {name}")

    return appdir, manifest


def validate_active_launcher(prefix: Path) -> str | None:
    launcher = launcher_path(prefix)
    try:
        check_existing_parents(launcher.parent)
        contents, _ = read_regular(launcher, mode=0o644)
    except FileNotFoundError:
        return None
    app_root = prefix / "lib" / "inodune"
    checked_directory(app_root)
    matches: list[str] = []
    with os.scandir(app_root) as entries:
        for entry in entries:
            if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", entry.name):
                continue
            if entry.is_symlink() or not entry.is_dir(follow_symlinks=False):
                raise InstallRefused(f"Unexpected entry in Inodune install directory: {entry.path}")
            _, manifest = read_manifest(prefix, entry.name)
            if hashlib.sha256(contents).hexdigest() == manifest["launcher_sha256"]:
                matches.append(entry.name)
    if len(matches) != 1:
        raise InstallRefused("The desktop launcher is changed or does not point to one verified Inodune install")
    return matches[0]


def switch_launcher(prefix: Path, version: str, active_version: str | None) -> None:
    launcher = launcher_path(prefix)
    replacement = launcher_bytes(prefix, version)
    temporary = launcher.parent / f".inodune-launcher-{secrets.token_hex(12)}"
    create_exclusive(temporary, replacement, 0o644)
    try:
        if active_version is None:
            os.link(temporary, launcher, follow_symlinks=False)
        else:
            existing, original_info = read_regular(launcher, mode=0o644)
            if existing != launcher_bytes(prefix, active_version):
                raise InstallRefused("The active desktop launcher changed before update")
            parent_fd = os.open(launcher.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
            try:
                current = os.stat(launcher.name, dir_fd=parent_fd, follow_symlinks=False)
                if (current.st_dev, current.st_ino) != (original_info.st_dev, original_info.st_ino):
                    raise InstallRefused("The desktop launcher changed before update")
                os.rename(temporary.name, launcher.name, src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
            finally:
                os.close(parent_fd)
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def installed_versions(prefix: Path) -> list[str]:
    app_root = prefix / "lib" / "inodune"
    checked_directory(app_root)
    versions = []
    with os.scandir(app_root) as entries:
        for entry in entries:
            if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", entry.name):
                if entry.is_symlink() or not entry.is_dir(follow_symlinks=False):
                    raise InstallRefused(f"Unexpected version link: {entry.path}")
                read_manifest(prefix, entry.name)
                if read_uninstall_journal(prefix, entry.name) is None:
                    versions.append(entry.name)
    return sorted(versions, key=version_key)


def remove_active_launcher(prefix: Path, manifest: dict[str, object]) -> None:
    launcher = launcher_path(prefix)
    try:
        data, original_info = read_regular(launcher, mode=0o644)
    except FileNotFoundError:
        return
    if hashlib.sha256(data).hexdigest() != manifest["launcher_sha256"]:
        raise InstallRefused("The active launcher changed; refusing removal")
    launcher_fd = os.open(launcher.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        current = os.stat(launcher.name, dir_fd=launcher_fd, follow_symlinks=False)
        if (current.st_dev, current.st_ino) != (original_info.st_dev, original_info.st_ino):
            raise InstallRefused("The active launcher changed before removal")
        os.unlink(launcher.name, dir_fd=launcher_fd)
    finally:
        os.close(launcher_fd)


def remove_uninstall_journal(prefix: Path, version: str, manifest: dict[str, object]) -> None:
    path = uninstall_journal_path(prefix, version)
    journal = {"schema": 1, "version": version, "prefix": str(prefix), "manifest": manifest}
    expected = (json.dumps(journal, sort_keys=True, separators=(",", ":")) + "\n").encode()
    data, original_info = read_regular(path, mode=0o600)
    if data != expected:
        raise InstallRefused(f"Uninstall journal changed; preserved for inspection: {path}")
    parent_fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        current = os.stat(path.name, dir_fd=parent_fd, follow_symlinks=False)
        if (current.st_dev, current.st_ino) != (original_info.st_dev, original_info.st_ino):
            raise InstallRefused(f"Uninstall journal changed before removal: {path}")
        os.unlink(path.name, dir_fd=parent_fd)
    finally:
        os.close(parent_fd)


def uninstall(prefix: Path, version: str, *, apply: bool = False) -> LifecycleReport:
    require_unprivileged_user()
    prefix = prefix.expanduser()
    if not prefix.is_absolute() or ".." in prefix.parts:
        raise InstallRefused("--prefix must be an absolute path without parent components")
    appdir, manifest = read_manifest(prefix, version)
    journal_path = uninstall_journal_path(prefix, version)
    journal_exists = read_uninstall_journal(prefix, version) is not None
    versions = installed_versions(prefix)
    active = validate_active_launcher(prefix)
    fallback = max((item for item in versions if item != version), key=version_key, default=None) if active == version else None
    lines = [f"Version: {version}", f"Application directory: {appdir}"]
    if active == version:
        lines.append(f"Desktop launcher: switch to {fallback}" if fallback else "Desktop launcher: remove after its executable")
    try:
        appdir.lstat()
        directory_exists = True
    except FileNotFoundError:
        directory_exists = False
    if directory_exists:
        for name in ("bin/inodune-cli", "LICENSE", "inodune-icon-512.png", "bin/inodune-desktop"):
            if (appdir / name).exists() or (appdir / name).is_symlink():
                lines.append(f"Remove verified file: {appdir / name}")
        lines.append(f"Remove install manifest: {appdir / MANIFEST_NAME}")
    else:
        lines.append("Application directory is already removed; retry will finish journal cleanup")
    lines.append(f"Recovery journal: {journal_path}")
    if not apply:
        return LifecycleReport(applied=False, lines=lines)

    try:
        if not journal_exists:
            journal = {"schema": 1, "version": version, "prefix": str(prefix), "manifest": manifest}
            create_exclusive(journal_path, (json.dumps(journal, sort_keys=True, separators=(",", ":")) + "\n").encode(), 0o600)

        if active == version and fallback is not None:
            switch_launcher(prefix, fallback, active_version=version)

        if directory_exists:
            # Retain a valid launcher until the executable itself is removed.
            # The journal makes any completed earlier leaf unlink resumable.
            for name in ("bin/inodune-cli", "LICENSE", "inodune-icon-512.png", "bin/inodune-desktop"):
                path = appdir / name
                try:
                    path.lstat()
                except FileNotFoundError:
                    continue
                record = manifest["files"][name]
                data, original_info = read_regular(path, mode=APP_FILES[name], limit=64 * 1024 * 1024)
                if hashlib.sha256(data).hexdigest() != record["sha256"]:
                    raise InstallRefused(f"File changed before removal; preserved: {path}")
                parent = path.parent
                parent_info = checked_install_directory(parent)
                parent_fd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
                try:
                    opened_parent = os.fstat(parent_fd)
                    current = os.stat(path.name, dir_fd=parent_fd, follow_symlinks=False)
                    if (opened_parent.st_dev, opened_parent.st_ino) != (parent_info.st_dev, parent_info.st_ino):
                        raise InstallRefused(f"Install directory changed before removal; preserved: {parent}")
                    if (current.st_dev, current.st_ino) != (original_info.st_dev, original_info.st_ino):
                        raise InstallRefused(f"File changed before removal; preserved: {path}")
                    os.unlink(path.name, dir_fd=parent_fd)
                finally:
                    os.close(parent_fd)

                if name == "bin/inodune-desktop" and active == version and fallback is None:
                    remove_active_launcher(prefix, manifest)

            # A prior attempt may have removed the GUI binary but failed while
            # unlinking the launcher. Complete that exact state on retry.
            if active == version and fallback is None and not (appdir / "bin" / "inodune-desktop").exists():
                remove_active_launcher(prefix, manifest)

            try:
                (appdir / "bin").lstat()
            except FileNotFoundError:
                pass
            else:
                bin_info = checked_install_directory(appdir / "bin")
                app_fd = os.open(appdir, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
                try:
                    current_bin = os.stat("bin", dir_fd=app_fd, follow_symlinks=False)
                    if (current_bin.st_dev, current_bin.st_ino) != (bin_info.st_dev, bin_info.st_ino):
                        raise InstallRefused("Application bin directory changed before removal")
                    os.rmdir("bin", dir_fd=app_fd)
                finally:
                    os.close(app_fd)

            manifest_path = appdir / MANIFEST_NAME
            try:
                manifest_data, manifest_info = read_regular(manifest_path, mode=0o600)
            except FileNotFoundError:
                pass
            else:
                try:
                    current_manifest = json.loads(manifest_data)
                except (UnicodeDecodeError, json.JSONDecodeError) as error:
                    raise InstallRefused(f"Install manifest changed before removal: {error}") from error
                if current_manifest != manifest:
                    raise InstallRefused("Install manifest changed before removal; preserved")
                app_fd = os.open(appdir, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
                try:
                    current = os.stat(MANIFEST_NAME, dir_fd=app_fd, follow_symlinks=False)
                    if (current.st_dev, current.st_ino) != (manifest_info.st_dev, manifest_info.st_ino):
                        raise InstallRefused("Install manifest changed before removal")
                    os.unlink(MANIFEST_NAME, dir_fd=app_fd)
                finally:
                    os.close(app_fd)

            app_root_fd = os.open(appdir.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
            try:
                current_appdir = os.stat(appdir.name, dir_fd=app_root_fd, follow_symlinks=False)
                original_appdir = appdir.lstat()
                if (current_appdir.st_dev, current_appdir.st_ino) != (original_appdir.st_dev, original_appdir.st_ino):
                    raise InstallRefused("Application directory changed before removal")
                os.rmdir(appdir.name, dir_fd=app_root_fd)
            finally:
                os.close(app_root_fd)

        remove_uninstall_journal(prefix, version, manifest)
    except (OSError, InstallRefused) as error:
        lines.append(
            f"Partial uninstall; retry with --uninstall {version} --apply. "
            f"The journal preserves file checksums and already removed paths: {error}"
        )
        return LifecycleReport(applied=True, complete=False, lines=lines)
    return LifecycleReport(applied=True, lines=lines)


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
    require_unprivileged_user()
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
    launcher_dir = launcher_path(prefix).parent
    active_version = validate_active_launcher(prefix)

    if prefix.exists():
        info = prefix.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid():
            raise RuntimeError(f"--prefix must be an existing directory owned by this user: {prefix}")
    ensure_directory(prefix)
    ensure_directory(appdir.parent)
    ensure_directory(launcher_dir)

    if read_uninstall_journal(prefix, version) is not None:
        raise InstallRefused(f"Version {version} has an incomplete uninstall; resume it before installing that version")

    # A completed same-version retry is safe only when every payload hash matches.
    try:
        _, existing = read_manifest(prefix, version)
    except FileNotFoundError:
        existing = None
    if existing is not None:
        expected_files = {
            name: {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data), "mode": mode}
            for name, mode in APP_FILES.items()
            for data in (file_data[name.removeprefix("bin/")] if name.startswith("bin/") else file_data[name],)
        }
        if existing["files"] != expected_files:
            raise FileExistsError(f"Refusing to replace a different build with the same version: {appdir}")
        if active_version != version and (active_version is None or version_key(version) > version_key(active_version)):
            switch_launcher(prefix, version, active_version)
        return appdir

    stage_name = f".inodune-staging-{version}-{secrets.token_hex(12)}"
    app_root_fd = os.open(appdir.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    stage_identity: tuple[int, int] | None = None
    bin_identity: tuple[int, int] | None = None
    created: list[tuple[str, int, int]] = []
    try:
        os.mkdir(stage_name, mode=0o700, dir_fd=app_root_fd)
        stage_info = os.stat(stage_name, dir_fd=app_root_fd, follow_symlinks=False)
        stage_identity = (stage_info.st_dev, stage_info.st_ino)
        stage_dir = appdir.parent / stage_name
        stage_fd = os.open(stage_name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=app_root_fd)
        try:
            os.mkdir("bin", mode=0o755, dir_fd=stage_fd)
            bin_info = os.stat("bin", dir_fd=stage_fd, follow_symlinks=False)
            bin_identity = (bin_info.st_dev, bin_info.st_ino)
            bin_fd = os.open("bin", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=stage_fd)
            try:
                for name in SOURCES:
                    create_exclusive_at(bin_fd, name, f"bin/{name}", file_data[name], 0o755, created)
            finally:
                os.close(bin_fd)
            create_exclusive_at(stage_fd, "LICENSE", "LICENSE", file_data["LICENSE"], 0o644, created)
            create_exclusive_at(stage_fd, "inodune-icon-512.png", "inodune-icon-512.png", file_data["inodune-icon-512.png"], 0o644, created)
            create_exclusive_at(
                stage_fd,
                MANIFEST_NAME,
                MANIFEST_NAME,
                create_manifest(prefix, stage_dir, version),
                0o600,
                created,
            )
            os.fsync(stage_fd)
            read_manifest(prefix, version, appdir_override=stage_dir)
        finally:
            os.close(stage_fd)

        rename_noreplace_at(app_root_fd, stage_name, version)
        stage_identity = None
        os.fsync(app_root_fd)
    except BaseException as error:
        if stage_identity is not None:
            try:
                cleanup_staging_dir(app_root_fd, stage_name, stage_identity, bin_identity, created)
            except BaseException as cleanup_error:
                raise InstallRefused(
                    f"Install failed ({error}); incomplete staging data was preserved at {appdir.parent / stage_name} for inspection"
                ) from cleanup_error
        raise
    finally:
        os.close(app_root_fd)

    # The published directory is complete and hash-validated before the launcher changes.
    switch_launcher(prefix, version, active_version)
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
    parser.add_argument("--uninstall", metavar="VERSION", help="preview removal of one manifest-verified user installation")
    parser.add_argument("--apply", action="store_true", help="apply the previewed uninstall (otherwise it is a dry run)")
    if PORTABLE_BUNDLE and "--archive" in sys.argv:
        parser.error("use either an extracted portable bundle or --archive")
    args = parser.parse_args()
    if args.uninstall and args.archive:
        parser.error("--uninstall cannot be combined with --archive")
    if args.apply and not args.uninstall:
        parser.error("--apply is only valid with --uninstall VERSION")
    try:
        if args.uninstall:
            report = uninstall(args.prefix, args.uninstall, apply=args.apply)
            print("Applied uninstall:" if report.applied else "Uninstall dry run (no files changed):")
            for line in report.lines:
                print(f"  {line}")
            return 0 if report.complete else 1
        location = install(args.prefix, args.archive)
    except (FileExistsError, OSError, RuntimeError, KeyError, ValueError) as error:
        print(f"Inodune user install refused or incomplete: {error}", file=sys.stderr)
        return 1
    print(f"Installed Inodune under {location}")
    print(f"CLI: {location / 'bin' / 'inodune-cli'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
