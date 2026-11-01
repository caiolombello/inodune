"""Smoke the non-root installer only in task-created temporary prefixes."""
import configparser
import hashlib
import json
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
    version=json.loads((ROOT / 'apps/desktop/src-tauri/tauri.conf.json').read_text())['version']
    archive = ROOT / f'target/packages/{version}/Inodune_{version}_linux_x86_64.tar.gz'
    app = installer.install(prefix, archive)
    launcher = prefix / 'share/applications/inodune.desktop'
    parser = configparser.RawConfigParser()
    parser.read(launcher)
    assert Path(parser['Desktop Entry']['TryExec']).exists()
    assert Path(parser['Desktop Entry']['Icon']).is_file()
    subprocess.run(['desktop-file-validate', str(launcher)], check=True)
    subprocess.run([str(app / 'bin/inodune-cli'), '--help'], check=True, stdout=subprocess.DEVNULL)
    before = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in app.rglob('*') if p.is_file()}
    installer.install(prefix, archive)
    assert before == {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in before}
    print('PASS: private fixture install, valid desktop launcher including spaces/special characters, CLI help, same-version retry preserves all files.')
