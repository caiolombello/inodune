#!/usr/bin/env python3
"""Bundle Inodune DEB/RPMs in the project-local, rootless Debian 12 packager."""
from __future__ import annotations

import hashlib
import gzip
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
IMAGE = "localhost/inodune-packager:debian12"
TARGET = ROOT / "target" / "compat"
BINARY_DIR = TARGET / "release"
PACKAGE_DIR = ROOT / "target" / "packages"


def checked(args: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, check=True, text=True, **kwargs)


def prepare_snapshot(destination: Path) -> None:
    for file in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "LICENSE"):
        shutil.copyfile(ROOT / file, destination / file)
    shutil.copytree(ROOT / "crates", destination / "crates")
    shutil.copytree(ROOT / "tests" / "fixtures", destination / "tests" / "fixtures")
    shutil.copytree(ROOT / "assets" / "brand", destination / "assets" / "brand")
    shutil.copytree(
        ROOT / "apps" / "desktop" / "src-tauri",
        destination / "apps" / "desktop" / "src-tauri",
        ignore=shutil.ignore_patterns("target", "gen"),
    )
    shutil.copytree(ROOT / "apps" / "desktop" / "dist", destination / "apps" / "desktop" / "dist")
    release = destination / "target" / "compat" / "release"
    release.mkdir(parents=True)
    # The Tauri bundler patches its input binary with bundle metadata. Copy only
    # the two release artifacts into the isolated snapshot so it cannot patch
    # the build output or touch the source checkout.
    for name in ("inodune-desktop", "inodune-cli"):
        shutil.copyfile(BINARY_DIR / name, release / name)
        (release / name).chmod(0o755)
    (destination / "scripts").mkdir()
    shutil.copyfile(ROOT / "scripts" / "validate-linux-packages.py", destination / "scripts" / "validate-linux-packages.py")


def validate_inputs() -> None:
    for path in (BINARY_DIR / "inodune-desktop", BINARY_DIR / "inodune-cli"):
        if not path.is_file() or not os.access(path, os.X_OK):
            raise SystemExit(f"Missing executable build output: {path}")
    if not (ROOT / "apps" / "desktop" / "dist" / "index.html").is_file():
        raise SystemExit("Missing production frontend: apps/desktop/dist/index.html")
    if not (ROOT / "node_modules" / "@tauri-apps" / "cli" / "tauri.js").is_file():
        raise SystemExit("Install locked npm dependencies first (`npm ci`).")


def build() -> None:
    validate_inputs()
    if subprocess.run(["podman", "image", "exists", IMAGE]).returncode:
        checked(
            [
                "podman",
                "build",
                "--http-proxy=false",
                "-f",
                str(ROOT / "packaging" / "Containerfile.packager"),
                "-t",
                IMAGE,
                str(ROOT / "packaging"),
            ]
        )
    TARGET.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="inodune-package-source-") as temporary:
        snapshot = Path(temporary)
        prepare_snapshot(snapshot)
        args = [
            "podman",
            "run",
            "--rm",
            "--userns=keep-id:uid=1000,gid=1000",
            "--network=none",
            "--cap-drop=all",
            "--security-opt=no-new-privileges",
            "--security-opt=label=disable",
            "--env",
            "CARGO_TARGET_DIR=/workspace/target/compat",
            "--env",
            "RUSTUP_TOOLCHAIN=1.93.0",
            "--env",
            "XDG_CONFIG_HOME=/tmp/inodune-config",
            "--env",
            "XDG_CACHE_HOME=/tmp/inodune-cache",
            "--volume",
            f"{snapshot}:/workspace:rw",
            "--volume",
            f"{ROOT / 'node_modules'}:/workspace/node_modules:ro",
            "--workdir",
            "/workspace/apps/desktop",
            IMAGE,
            "/workspace/node_modules/@tauri-apps/cli/tauri.js",
            "bundle",
            "--bundles",
            "deb,rpm",
            "--ci",
            "--no-sign",
            "--features",
            "custom-protocol",
        ]
        checked(args)
        bundle_root = snapshot / "target" / "compat" / "release" / "bundle"
        debs = sorted((bundle_root / "deb").glob("*.deb"))
        rpms = sorted((bundle_root / "rpm").glob("*.rpm"))
        if len(debs) != 1 or len(rpms) != 1:
            raise SystemExit(f"Expected one package of each type; found {len(debs)} DEB and {len(rpms)} RPM")
        validate_packages(snapshot, debs[0], rpms[0])
        install_output(debs[0], rpms[0])


def validate_packages(snapshot: Path, deb: Path, rpm: Path) -> None:
    checked(
        [
            "podman",
            "run",
            "--rm",
            "--userns=keep-id:uid=1000,gid=1000",
            "--network=none",
            "--cap-drop=all",
            "--security-opt=no-new-privileges",
            "--security-opt=label=disable",
            "--volume",
            f"{snapshot}:/workspace:ro",
            "--workdir",
            "/workspace",
            "--entrypoint",
            "/usr/bin/python3",
            IMAGE,
            "/workspace/scripts/validate-linux-packages.py",
            "/workspace/target/compat/release/bundle/deb/" + deb.name,
            "/workspace/target/compat/release/bundle/rpm/" + rpm.name,
        ]
    )


def install_output(deb: Path, rpm: Path) -> None:
    PACKAGE_DIR.mkdir(parents=True, exist_ok=True)
    deb_out = PACKAGE_DIR / deb.name
    rpm_out = PACKAGE_DIR / rpm.name
    shutil.copyfile(deb, deb_out)
    shutil.copyfile(rpm, rpm_out)
    portable = create_portable_archive()
    installer = PACKAGE_DIR / "install-user.py"
    shutil.copyfile(ROOT / "scripts" / "install-user.py", installer)
    checked([sys.executable, str(ROOT / "scripts" / "generate-sbom.py")])
    sums = PACKAGE_DIR / "SHA256SUMS"
    entries = []
    for path in (
        deb_out,
        rpm_out,
        portable,
        installer,
        PACKAGE_DIR / "inodune-frontend.spdx.json",
        PACKAGE_DIR / "inodune-rust.spdx.json",
    ):
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        entries.append(f"{digest}  {path.name}")
    sums.write_text("\n".join(entries) + "\n", encoding="utf-8")
    print(f"Packages and local SHA-256 checksums written to {PACKAGE_DIR}")


def create_portable_archive() -> Path:
    version = json.loads(
        (ROOT / "apps" / "desktop" / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8")
    )["version"]
    name = f"Inodune_{version}_linux_x86_64.tar.gz"
    output = PACKAGE_DIR / name
    files = {
        "install-user.py": (ROOT / "scripts" / "install-user.py", 0o755),
        "VERSION": None,
        "bin/inodune-desktop": (BINARY_DIR / "inodune-desktop", 0o755),
        "bin/inodune-cli": (BINARY_DIR / "inodune-cli", 0o755),
        "LICENSE": (ROOT / "LICENSE", 0o644),
        "inodune-icon-512.png": (ROOT / "assets" / "brand" / "inodune-icon-512.png", 0o644),
    }
    with output.open("wb") as compressed_file:
        with gzip.GzipFile(filename="", mode="wb", fileobj=compressed_file, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as archive:
                for member_name, source in files.items():
                    if source is None:
                        content = (version + "\n").encode("utf-8")
                        mode = 0o644
                    else:
                        source_path, mode = source
                        content = source_path.read_bytes()
                    info = tarfile.TarInfo(member_name)
                    info.size = len(content)
                    info.mode = mode
                    info.mtime = 0
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    archive.addfile(info, io.BytesIO(content))
    return output


if __name__ == "__main__":
    build()
