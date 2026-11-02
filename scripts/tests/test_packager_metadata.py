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
        # Placeholder files in a tempdir; no real .deb/.rpm is generated.
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            for name in ('artifact.deb', 'artifact.rpm', 'artifact.tar.gz', 'install-user.py',
                         'inodune-frontend.spdx.json', 'inodune-rust.spdx.json'):
                (out / name).write_bytes(name.encode())
            with patch.object(module, 'OUT', out):
                result = module.write_checksums()
            lines = result.read_text().splitlines()
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
        # Tauri's `changelog` option writes /usr/share/doc/<productName>/
        # (Inodune), not the package directory: ship it next to copyright.
        self.assertNotIn('changelog', deb)
        changelog = ROOT / 'packaging/debian/changelog'
        packed = (base / deb['files']['/usr/share/doc/inodune/changelog.gz']).resolve()
        copyright_file = (base / deb['files']['/usr/share/doc/inodune/copyright']).resolve()
        import gzip
        self.assertEqual(gzip.decompress(packed.read_bytes()), changelog.read_bytes(),
                         'regenerate packaging/debian/changelog.gz (gzip -9n)')
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


class ChecksumListingTests(unittest.TestCase):
    def test_sha256sums_lists_only_files_present_in_the_output_folder(self):
        module = load('generate-sbom')
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            for name in ('Inodune_0.1.11_amd64.deb', 'Inodune-0.1.11-1.x86_64.rpm',
                         'Inodune_0.1.11_linux_x86_64.tar.gz', *module.REQUIRED_OUTPUTS):
                (out / name).write_bytes(name.encode())
            (out / 'SHA256SUMS').write_text('stale  removed-artifact.deb\n')
            (out / 'subdir').mkdir()
            with patch.object(module, 'OUT', out):
                sums = module.write_checksums()
            names = [line.split('  ', 1)[1] for line in sums.read_text().splitlines()]
            present = sorted(p.name for p in out.iterdir() if p.is_file() and p.name != 'SHA256SUMS')
            self.assertEqual(names, present)
            for line in sums.read_text().splitlines():
                digest, name = line.split('  ', 1)
                self.assertEqual(digest, hashlib.sha256((out / name).read_bytes()).hexdigest())
            (out / 'inodune-rust.spdx.json').unlink()
            with patch.object(module, 'OUT', out), self.assertRaises(RuntimeError):
                module.write_checksums()


class LintianWarningTests(unittest.TestCase):
    """lintian W: extended-description-line-too-long and no-manual-page."""

    def test_short_description_lines_and_compressed_man_pages(self):
        import gzip
        import json
        conf = json.loads((ROOT / 'apps/desktop/src-tauri/tauri.conf.json').read_text())
        self.assertLess(len(conf['bundle']['longDescription']), 80)
        self.assertNotIn('\n', conf['bundle']['longDescription'])
        files = conf['bundle']['linux']['deb']['files']
        base = ROOT / 'apps/desktop/src-tauri'
        for binary in ('inodune-desktop', 'inodune-cli', 'inodune-uninstall-helper'):
            packed = (base / files[f'/usr/share/man/man1/{binary}.1.gz']).resolve()
            source = ROOT / f'packaging/debian/man/{binary}.1'
            self.assertEqual(gzip.decompress(packed.read_bytes()), source.read_bytes(),
                             f'regenerate {binary}.1.gz (gzip -9n)')
            self.assertTrue(source.read_text().startswith('.TH '))
