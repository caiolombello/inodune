#!/usr/bin/env python3
"""Run the ignored ownership-boundary integration lane in disposable Fedora 44."""
import glob
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/evidence/f4/fedora-44/runtime-smoke.json"
RUSTUP_HOME = ROOT / ".tools/rustup"
CARGO_HOME = ROOT / ".tools/cargo"


def main() -> int:
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    image_id = evidence["localImageId"]
    subprocess.run(
        ["podman", "image", "exists", image_id], check=True, cwd=ROOT
    )
    env = os.environ.copy()
    env["RUSTUP_HOME"] = str(RUSTUP_HOME)
    env["CARGO_HOME"] = str(CARGO_HOME)
    subprocess.run(
        [str(CARGO_HOME / "bin/cargo"), "test", "-p", "inodune-core", "--test", "discovery-ownership", "--no-run"],
        check=True,
        cwd=ROOT,
        env=env,
    )
    binaries = sorted(
        (Path(p) for p in glob.glob(str(ROOT / "target/debug/deps/discovery_ownership-*"))),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    binary = next((p for p in binaries if p.is_file() and os.access(p, os.X_OK)), None)
    if binary is None:
        raise RuntimeError("cargo built no discovery-ownership integration test binary")

    setup_and_run = r'''
import os, pathlib, shutil, sys
base = pathlib.Path('/tmp/inodune-owned-boundary-fixture')
shutil.rmtree(base, ignore_errors=True)
home = base / 'home'
structural = base / 'structural'
project = structural / 'owned-project'
foreign = structural / 'foreign-private'
outside = base / 'outside'
escape = outside / 'escape-project'
for p in (home, project, foreign, escape): p.mkdir(parents=True, exist_ok=True)
(project / 'marker').write_bytes(b'task-created fixture sentinel')
(outside / 'sentinel').write_bytes(b'external target')
os.chown(home, 1000, 1000)
os.chmod(home, 0o700)
os.chown(project, 1000, 1000)
os.chmod(project, 0o755)
os.chown(project / 'marker', 1000, 1000)
os.chmod(project / 'marker', 0o644)
os.chown(foreign, 1001, 1001)
os.chmod(foreign, 0o700)
os.chown(outside, 0, 0)
os.chmod(outside, 0o755)
os.chown(escape, 1000, 1000)
os.chmod(escape, 0o755)
os.symlink(outside, structural / 'outside-link')
os.chmod(structural, 0o755)
test_binary = pathlib.Path('/tmp/discovery-ownership')
shutil.copy2(sys.argv[1], test_binary)
test_binary.chmod(0o755)
pid = os.fork()
if pid == 0:
    os.setgid(1000); os.setuid(1000)
    os.environ['HOME'] = str(home)
    os.environ['INODUNE_DISCOVERY_OWNERSHIP_FIXTURE'] = str(base)
    os.execv(test_binary, [str(test_binary), '--exact', 'ownership_boundaries_symlinks_noatime_limits_and_cancel_are_conservative', '--ignored', '--nocapture'])
_, status = os.waitpid(pid, 0)
sys.exit(os.waitstatus_to_exitcode(status))
'''
    command = [
        "podman", "run", "--rm", "--network=none", "--user=0:0",
        "--volume", f"{binary}:/inodune-test:ro,Z", "--security-opt", "label=disable",
        "--entrypoint", "python3", image_id, "-c", setup_and_run, "/inodune-test",
    ]
    return subprocess.run(command, cwd=ROOT).returncode


if __name__ == "__main__":
    sys.exit(main())
