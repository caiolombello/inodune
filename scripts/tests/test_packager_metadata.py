"""No container, Cargo, npm or package generation: only orchestration and hashes."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch

ROOT = Path(__file__).resolve().parents[2]
def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

class PackagerMetadataTests(unittest.TestCase):
    def test_rust_and_checksum_command_runs_only_in_offline_packager(self):
        module = load('package-linux')
        with tempfile.TemporaryDirectory() as directory:
            registry = Path(directory) / '.tools/cargo/registry'
            registry.mkdir(parents=True)
            with patch.object(module, 'ROOT', Path(directory)), patch.object(module, 'checked') as run:
                with patch.dict('os.environ', {}, clear=True):
                    module.generate_container_metadata(Path(directory) / 'snapshot')
            args = run.call_args.args[0]
            self.assertEqual(args[0], 'podman')
            self.assertIn('--network=none', args)
            self.assertIn('localhost/inodune-packager:debian12', args)
            self.assertIn('/usr/bin/python3', args)
            self.assertIn('INODUNE_PACKAGER=1', args)
            self.assertIn('--rust-only', args)
            self.assertIn('--checksums', args)
            self.assertNotIn('cargo', args)

    def test_host_frontend_path_does_not_invoke_cargo(self):
        module = load('generate-sbom')
        output = MagicMock()
        with patch.object(module, 'OUT', MagicMock()), patch.object(module, 'generate_frontend', return_value=output), patch.object(module, 'generate_rust') as rust:
            with patch('sys.argv', ['generate-sbom.py', '--frontend-only']), patch.dict('os.environ', {}, clear=True):
                module.main()
            rust.assert_not_called()
        with patch('sys.argv', ['generate-sbom.py']), patch.dict('os.environ', {}, clear=True):
            with self.assertRaises(SystemExit):
                module.main()

    def test_checksums_cover_both_sboms_and_all_distribution_artifacts(self):
        module = load('generate-sbom')
        # Mock paths prevent generating even dummy .deb/.rpm files.
        class File:
            def __init__(self, name):
                self.name = name
            def read_bytes(self):
                return self.name.encode()
            def write_text(self, text, **kwargs):
                self.text = text
        class Output:
            def __init__(self):
                self.files = {}
            def glob(self, suffix):
                return [self['artifact' + suffix[1:]]]
            def __getitem__(self, name):
                return self.files.setdefault(name, File(name))
            __truediv__ = __getitem__
        out = Output()
        with patch.object(module, 'OUT', out):
            result = module.write_checksums()
        lines = result.text.splitlines()
        self.assertEqual(len(lines), 6)
        for line in lines:
            digest, name = line.split('  ')
            self.assertEqual(digest, hashlib.sha256(name.encode()).hexdigest())
        self.assertTrue(any('inodune-rust.spdx.json' in line for line in lines))
        self.assertTrue(any('inodune-frontend.spdx.json' in line for line in lines))

if __name__ == '__main__':
    unittest.main()


class DebianControlMetadataTests(unittest.TestCase):
    """Lintian: malformed-contact, missing-dependency-on-libc, no-changelog, no-copyright-file."""

    def test_deb_metadata_has_contact_libc_changelog_and_copyright(self):
        import json
        import re
        conf = json.loads((ROOT / 'apps/desktop/src-tauri/tauri.conf.json').read_text())
        deb = conf['bundle']['linux']['deb']
        cargo = (ROOT / 'apps/desktop/src-tauri/Cargo.toml').read_text()
        contact = re.compile(r'^[^<>]+ <[^<>@\s]+@[^<>@\s]+>$')
        authors = re.search(r'^authors = \["([^"]+)"\]', cargo, re.M)
        self.assertIsNotNone(authors)
        self.assertRegex(authors.group(1), contact)
        self.assertRegex(conf['bundle']['publisher'], contact)
        self.assertIn('noreply.github.com', authors.group(1))
        self.assertTrue(conf['bundle']['homepage'].startswith('https://'))
        self.assertEqual(deb['section'], 'utils')
        self.assertTrue(any(d.startswith('libc6 (>= ') for d in deb['depends']))
        base = ROOT / 'apps/desktop/src-tauri'
        changelog = (base / deb['changelog']).resolve()
        copyright_file = (base / deb['files']['/usr/share/doc/inodune/copyright']).resolve()
        self.assertTrue(changelog.is_file())
        self.assertTrue(copyright_file.is_file())
        first = changelog.read_text().splitlines()[0]
        self.assertEqual(first.split()[:2], ['inodune', f"({conf['version']})"])
        trailer = [l for l in changelog.read_text().splitlines() if l.startswith(' -- ')]
        self.assertRegex(trailer[0][4:].split('  ')[0], contact)
        self.assertTrue(copyright_file.read_text().startswith('Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/'))

    def test_packager_snapshot_includes_debian_metadata(self):
        import inspect
        module = load('package-linux')
        source = inspect.getsource(module.prepare_snapshot)
        self.assertIn('ROOT / "packaging" / "debian"', source)
