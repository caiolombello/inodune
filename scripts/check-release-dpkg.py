"""Check a default-feature release binary without removing any package."""
import os
import subprocess
import sys
from pathlib import Path

binary = Path(sys.argv[1]).resolve()
marker = 'INODUNE_CONTAINER_DPKG'
assert marker.encode() not in binary.read_bytes(), 'test shortcut present in release'
# This deliberately absent inventory ID cannot reach a removal transaction.
args = [str(binary), 'apps', 'uninstall', 'inodune-release-check-absent-7cfe3e91', '--yes']
env = dict(os.environ)
env.pop(marker, None)
normal = subprocess.run(args, env=env, capture_output=True)
env[marker] = '1'
with_marker = subprocess.run(args, env=env, capture_output=True)
assert normal.returncode != 0
assert (normal.returncode, normal.stdout, normal.stderr) == (
    with_marker.returncode, with_marker.stdout, with_marker.stderr
), 'test variable changed release behavior'
assert b'dpkg direto' not in with_marker.stderr
print('PASS: shortcut string absent; variable ignored; absent ID refused')
