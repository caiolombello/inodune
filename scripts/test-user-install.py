"""Smoke the non-root installer only in task-created temporary prefixes."""
import configparser
import hashlib
import importlib.util
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('install_user', ROOT / 'scripts/install-user.py')
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)
with tempfile.TemporaryDirectory(prefix='inodune-install-test-') as temporary:
    prefix = Path(temporary) / 'my apps $100%'
    archive = ROOT / 'target/packages/Inodune_0.1.0_linux_x86_64.tar.gz'
    app = installer.install(prefix, archive)
    launcher = prefix / 'share/applications/inodune-0.1.0.desktop'
    parser = configparser.RawConfigParser()
    parser.read(launcher)
    assert Path(parser['Desktop Entry']['TryExec']).exists()
    assert Path(parser['Desktop Entry']['Icon']).is_file()
    subprocess.run(['desktop-file-validate', str(launcher)], check=True)
    subprocess.run([str(app / 'bin/inodune-cli'), '--help'], check=True, stdout=subprocess.DEVNULL)
    before = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in app.rglob('*') if p.is_file()}
    try:
        installer.install(prefix, archive)
        raise AssertionError('conflicting install accepted')
    except FileExistsError:
        pass
    assert before == {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in before}
    print('PASS: private fixture install, valid desktop launcher including spaces/special characters, CLI help, conflict preserves all files.')
