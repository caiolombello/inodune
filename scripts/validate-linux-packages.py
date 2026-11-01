#!/usr/bin/env python3
"""Inspect and extract package artifacts inside the isolated Debian packager."""
from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path


def run(args: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, check=True, text=True, **kwargs)


def extract_rpm(rpm: Path, destination: Path) -> None:
    archive = destination.parent / "rpm-payload.cpio"
    with archive.open("wb") as payload:
        source = subprocess.run(["rpm2cpio", str(rpm)], stdout=payload, check=False)
    # rpm2cpio in Debian 12 returns status 1 after emitting a complete cpio
    # stream for this Fedora-compatible package. Validate the archive trailer
    # and let cpio parse the full archive before accepting the extracted tree.
    with archive.open("rb") as payload:
        payload.seek(-512, os.SEEK_END)
        trailer = payload.read()
    if source.returncode != 0 and b"TRAILER!!!" not in trailer:
        raise SystemExit("rpm2cpio failed before completing its archive")
    with archive.open("rb") as payload:
        run(
            ["cpio", "--extract", "--make-directories", "--no-absolute-filenames"],
            stdin=payload,
            stdout=subprocess.DEVNULL,
            cwd=destination,
        )
    archive.unlink()


def check_tree(root: Path) -> None:
    gui = root / "usr" / "bin" / "inodune-desktop"
    cli = root / "usr" / "bin" / "inodune-cli"
    if not gui.is_file() or not os.access(gui, os.X_OK):
        raise SystemExit(f"Desktop executable missing or not executable: {gui}")
    if not cli.is_file() or not os.access(cli, os.X_OK):
        raise SystemExit(f"CLI executable missing or not executable: {cli}")
    helper = root / "usr/bin/inodune-uninstall-helper"
    if not helper.is_file() or not os.access(helper, os.X_OK):
        raise SystemExit("Helper de desinstalação ausente ou não executável")
    license_file = root / "usr" / "share" / "licenses" / "inodune" / "LICENSE"
    if not license_file.is_file() or "MIT License" not in license_file.read_text(encoding="utf-8"):
        raise SystemExit("Application license missing from package")
    launchers = list((root / "usr" / "share" / "applications").glob("*.desktop"))
    if not launchers:
        raise SystemExit("Desktop launcher missing")
    icons = list((root / "usr" / "share" / "icons").rglob("*.png"))
    if not icons:
        raise SystemExit("Application icon missing")
    for launcher in launchers:
        content = launcher.read_text(encoding="utf-8")
        if "Exec=inodune-desktop" not in content:
            raise SystemExit(f"Launcher has an unexpected executable: {launcher}")
    run([str(cli), "--help"], stdout=subprocess.DEVNULL)
    print(f"PASS extracted package: {root}")


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: validate-linux-packages.py PACKAGE.deb PACKAGE.rpm")
    deb, rpm = map(Path, sys.argv[1:])
    run(["dpkg-deb", "--info", str(deb)])
    run(["dpkg-deb", "--field", str(deb), "Package", "Version", "Architecture", "Depends"])
    run(["rpm", "-qp", "--info", str(rpm)])
    run(["rpm", "-qp", "--requires", str(rpm)])
    with tempfile.TemporaryDirectory(prefix="inodune-package-fixture-") as temporary:
        base = Path(temporary)
        deb_root = base / "deb"
        rpm_root = base / "rpm"
        deb_root.mkdir()
        rpm_root.mkdir()
        run(["dpkg-deb", "--extract", str(deb), str(deb_root)])
        extract_rpm(rpm, rpm_root)
        check_tree(deb_root)
        check_tree(rpm_root)


if __name__ == "__main__":
    main()
