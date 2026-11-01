#!/usr/bin/env python3
"""Exercise user installation, upgrade, and uninstall only in TempDirs."""

from __future__ import annotations

import configparser
import errno
import hashlib
import importlib.util
import io
import os
import subprocess
import tarfile
import tempfile
import unittest
from unittest import mock
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("install_user", ROOT / "scripts" / "install-user.py")
assert SPEC is not None and SPEC.loader is not None
INSTALLER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INSTALLER)


def make_archive(path: Path, version: str, marker: bytes) -> Path:
    files = {
        "VERSION": version.encode() + b"\n",
        "LICENSE": b"fixture license\n",
        "inodune-icon-512.png": b"\x89PNG\r\n\x1a\nfixture",
        "bin/inodune-desktop": b"\x7fELF" + bytes(14) + (62).to_bytes(2, "little") + marker,
        "bin/inodune-cli": b"\x7fELF" + bytes(14) + (62).to_bytes(2, "little") + marker,
        "install-user.py": b"fixture installer\n",
    }
    with tarfile.open(path, "w:gz") as archive:
        for name, data in files.items():
            member = tarfile.TarInfo(name)
            member.size = len(data)
            member.mode = 0o755 if name.startswith("bin/") else 0o644
            archive.addfile(member, io.BytesIO(data))
    return path


class UserInstallLifecycleTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="inodune-user-lifecycle-")
        self.addCleanup(self.temporary.cleanup)
        base = Path(self.temporary.name)
        self.prefix = base / "apps with spaces"
        self.first_archive = make_archive(base / "first.tar.gz", "0.1.1", b"old")
        self.second_archive = make_archive(base / "second.tar.gz", "0.2.0", b"new")

    def install(self, archive: Path) -> Path:
        return INSTALLER.install(self.prefix, archive)

    def test_install_and_upgrade_switch_launcher_atomically_and_preserve_old_version(self) -> None:
        first = self.install(self.first_archive)
        second = self.install(self.second_archive)
        self.assertTrue((first / "bin" / "inodune-desktop").is_file())
        self.assertEqual((first / "bin" / "inodune-desktop").read_bytes()[-3:], b"old")
        self.assertEqual((second / "bin" / "inodune-desktop").read_bytes()[-3:], b"new")
        launcher = self.prefix / "share" / "applications" / "inodune.desktop"
        entry = configparser.RawConfigParser()
        entry.read(launcher)
        self.assertEqual(entry["Desktop Entry"]["TryExec"], str(second / "bin" / "inodune-desktop"))

    def test_same_version_retry_is_idempotent(self) -> None:
        app = self.install(self.first_archive)
        before = {p.relative_to(app): hashlib.sha256(p.read_bytes()).digest() for p in app.rglob("*") if p.is_file()}
        self.assertEqual(self.install(self.first_archive), app)
        after = {p.relative_to(app): hashlib.sha256(p.read_bytes()).digest() for p in app.rglob("*") if p.is_file()}
        self.assertEqual(before, after)

    def test_uninstall_dry_run_keeps_everything_and_apply_removes_only_verified_install(self) -> None:
        app = self.install(self.first_archive)
        before = {p.relative_to(self.prefix): hashlib.sha256(p.read_bytes()).digest() for p in self.prefix.rglob("*") if p.is_file()}
        preview = INSTALLER.uninstall(self.prefix, "0.1.1")
        self.assertFalse(preview.applied)
        self.assertTrue((app / "bin" / "inodune-desktop").exists())
        self.assertEqual(before, {p.relative_to(self.prefix): hashlib.sha256(p.read_bytes()).digest() for p in self.prefix.rglob("*") if p.is_file()})
        report = INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertTrue(report.applied)
        self.assertFalse(app.exists())

    def test_uninstall_refuses_to_remove_a_modified_file(self) -> None:
        app = self.install(self.first_archive)
        binary = app / "bin" / "inodune-cli"
        binary.write_bytes(b"changed by the user")
        with self.assertRaises(INSTALLER.InstallRefused):
            INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertTrue(binary.exists())
        self.assertTrue((app / "bin" / "inodune-desktop").exists())

    def test_uninstall_refuses_unknown_files_and_linked_installed_files(self) -> None:
        app = self.install(self.first_archive)
        unexpected = app / "notes.txt"
        unexpected.write_text("keep", encoding="utf-8")
        with self.assertRaises(INSTALLER.InstallRefused):
            INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        unexpected.unlink()
        external = Path(self.temporary.name) / "outside.txt"
        external.write_text("outside", encoding="utf-8")
        binary = app / "bin" / "inodune-cli"
        binary.unlink()
        binary.symlink_to(external)
        with self.assertRaises(INSTALLER.InstallRefused):
            INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertEqual(external.read_text(encoding="utf-8"), "outside")

    def test_uninstall_keeps_new_launcher_when_removing_old_version(self) -> None:
        self.install(self.first_archive)
        self.install(self.second_archive)
        new_launcher = self.prefix / "share" / "applications" / "inodune.desktop"
        before = new_launcher.read_bytes()
        INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertEqual(new_launcher.read_bytes(), before)
        self.assertTrue((self.prefix / "lib" / "inodune" / "0.2.0").exists())

    def test_cli_uninstall_defaults_to_preview_and_requires_apply(self) -> None:
        self.install(self.first_archive)
        completed = subprocess.run(
            ["python3", str(ROOT / "scripts" / "install-user.py"), "--prefix", str(self.prefix), "--uninstall", "0.1.1"],
            capture_output=True,
            text=True,
            check=True,
        )
        self.assertIn("dry run", completed.stdout.lower())
        self.assertTrue((self.prefix / "lib" / "inodune" / "0.1.1").exists())

    def test_uninstall_refuses_hardlinked_install_files(self) -> None:
        app = self.install(self.first_archive)
        binary = app / "bin" / "inodune-cli"
        outside = Path(self.temporary.name) / "linked-copy"
        os.link(binary, outside)
        with self.assertRaises(INSTALLER.InstallRefused):
            INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertTrue(binary.exists())
        self.assertTrue(outside.exists())

    def test_partial_filesystem_failure_is_reported_as_incomplete(self) -> None:
        app = self.install(self.first_archive)
        original_unlink = os.unlink

        def fail_on_cli(name: str | bytes, *args: object, **kwargs: object) -> None:
            if name == "inodune-cli":
                raise OSError("fixture interruption")
            original_unlink(name, *args, **kwargs)

        with mock.patch.object(INSTALLER.os, "unlink", side_effect=fail_on_cli):
            report = INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertTrue(report.applied)
        self.assertFalse(report.complete)
        self.assertTrue((app / "bin" / "inodune-cli").exists())
        self.assertIn("Partial uninstall", report.lines[-1])

    def test_modified_launcher_blocks_uninstall_before_touching_application(self) -> None:
        app = self.install(self.first_archive)
        launcher = self.prefix / "share" / "applications" / "inodune.desktop"
        launcher.write_text("user-edited launcher", encoding="utf-8")
        with self.assertRaises(INSTALLER.InstallRefused):
            INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertTrue((app / "bin" / "inodune-desktop").exists())
        self.assertEqual(launcher.read_text(encoding="utf-8"), "user-edited launcher")

    def test_install_and_uninstall_refuse_root_execution(self) -> None:
        with mock.patch.object(INSTALLER.os, "geteuid", return_value=0):
            with self.assertRaises(INSTALLER.InstallRefused):
                INSTALLER.install(self.prefix, self.first_archive)
            with self.assertRaises(INSTALLER.InstallRefused):
                INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)

    def test_install_enospc_cleans_only_staging_files_and_retry_succeeds(self) -> None:
        writes = 0
        real_write = os.write

        def fail_second_write(fd: int, data: bytes) -> int:
            nonlocal writes
            writes += 1
            if writes == 2:
                raise OSError(errno.ENOSPC, "fixture disk full")
            return real_write(fd, data)

        with mock.patch.object(INSTALLER.os, "write", side_effect=fail_second_write):
            with self.assertRaises(OSError):
                self.install(self.first_archive)
        app_root = self.prefix / "lib" / "inodune"
        self.assertFalse((app_root / "0.1.1").exists())
        self.assertFalse(any(p.name.startswith(".inodune-staging-") for p in app_root.iterdir()))
        self.assertFalse((self.prefix / "share" / "applications" / "inodune.desktop").exists())
        installed = self.install(self.first_archive)
        self.assertTrue((installed / "bin" / "inodune-desktop").is_file())

    def test_active_launcher_survives_first_unlink_failure_and_retry_completes(self) -> None:
        app = self.install(self.first_archive)
        launcher = self.prefix / "share" / "applications" / "inodune.desktop"
        original_launcher = launcher.read_bytes()
        original_unlink = os.unlink

        def fail_first_file(name: str | bytes, *args: object, **kwargs: object) -> None:
            if name == "inodune.desktop" or name == "inodune-cli":
                raise OSError(errno.EIO, "fixture interrupted uninstall")
            original_unlink(name, *args, **kwargs)

        with mock.patch.object(INSTALLER.os, "unlink", side_effect=fail_first_file):
            report = INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertFalse(report.complete)
        self.assertTrue((app / "bin" / "inodune-desktop").is_file())
        self.assertEqual(launcher.read_bytes(), original_launcher)
        completed = INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertTrue(completed.complete)
        self.assertFalse(app.exists())
        self.assertFalse(launcher.exists())

    def test_launcher_unlink_failure_after_executable_removal_is_resumable(self) -> None:
        app = self.install(self.first_archive)
        launcher = self.prefix / "share" / "applications" / "inodune.desktop"
        original_launcher = launcher.read_bytes()
        original_unlink = os.unlink

        def fail_launcher_once(name: str | bytes, *args: object, **kwargs: object) -> None:
            if name == "inodune.desktop":
                raise OSError(errno.EIO, "fixture interruption removing launcher")
            original_unlink(name, *args, **kwargs)

        with mock.patch.object(INSTALLER.os, "unlink", side_effect=fail_launcher_once):
            report = INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertFalse(report.complete)
        self.assertFalse((app / "bin" / "inodune-desktop").exists())
        self.assertEqual(launcher.read_bytes(), original_launcher)
        self.assertTrue((self.prefix / "lib" / "inodune" / ".inodune-uninstall-0.1.1.json").is_file())

        completed = INSTALLER.uninstall(self.prefix, "0.1.1", apply=True)
        self.assertTrue(completed.complete)
        self.assertFalse(app.exists())
        self.assertFalse(launcher.exists())


if __name__ == "__main__":
    unittest.main()
