"""T1280: offline pinned workspace notices for published crates."""
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tarfile
import tempfile
import tomllib
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'packaging/appimage'))
import sources
import crate_supplements


class SupplementalNoticesTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='blent-supplements-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / 'accesskit'
        shutil.copytree(REPO / 'scripts/tests/fixtures/crate-notices/accesskit', self.source)
        self.archives = self.root / 'archives'
        self.archives.mkdir()
        self.notices = self.root / 'notices'
        self.notices.mkdir()
        self.package = dict(name='accesskit', version='0.19.0',
                            source='registry+https://github.com/rust-lang/crates.io-index',
                            license='MIT OR Apache-2.0', license_file=None,
                            manifest_path=str(self.source / 'Cargo.toml'))
        self.catalog = self.root / 'catalog'
        shutil.copytree(crate_supplements.CATALOG.parent, self.catalog)
        self.catalog_patch = patch.object(crate_supplements, 'CATALOG', self.catalog / 'manifest.json')
        self.catalog_patch.start()
        self.addCleanup(self.catalog_patch.stop)

    def collect(self):
        return sources.copy_crate(self.package, self.archives, self.notices)

    def mutate_catalog(self, change):
        path = self.catalog / 'manifest.json'
        catalog = json.loads(path.read_text())
        change(catalog)
        path.write_text(json.dumps(catalog))

    def test_t1280_pinned_workspace_notices_build_offline_without_cache_mutation(self):
        before = {p.name: p.read_bytes() for p in self.source.iterdir()}
        record = sources.copy_crate(self.package, self.archives, self.notices)
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.source.iterdir()})
        self.assertEqual(len(record['notices']), 2)
        with tarfile.open(self.archives / 'accesskit-0.19.0.tar.gz') as archive:
            for row in record['notices']:
                with archive.extractfile('accesskit-0.19.0/' + row['path']) as stream:
                    self.assertEqual(stream.read(), (self.notices / 'accesskit-0.19.0' / row['path']).read_bytes())
                self.assertEqual(row['provenance']['revision'], 'daf5fafd63c42f62cc824a1f72e17aff1d102108')

    def test_t1280_identity_mismatch_never_publishes_notices(self):
        for field, value in [('version', '0.19.1'), ('source', 'registry+foreign'),
                             ('license', 'Apache-2.0'), ('license_file', 'LICENSE')]:
            with self.subTest(field=field):
                package = dict(self.package, **{field: value})
                with self.assertRaisesRegex(ValueError, 'pin|identity'):
                    sources.copy_crate(package, self.archives, self.notices)
        self.assertEqual(list(self.notices.iterdir()), [])
        self.assertEqual(list(self.archives.iterdir()), [])

    def test_t1280_changed_manifest_or_revision_is_rejected(self):
        manifest = self.source / 'Cargo.toml'
        original = manifest.read_bytes()
        manifest.write_bytes(original + b'\n')
        with self.assertRaisesRegex(ValueError, 'manifest mismatch'):
            self.collect()
        manifest.write_bytes(original)
        vcs = self.source / '.cargo_vcs_info.json'
        vcs.write_text('{"git":{"sha1":"wrong revision"}}')
        with self.assertRaisesRegex(ValueError, 'revision mismatch'):
            self.collect()

    def test_t1280_absent_or_corrupt_supplements_fail_closed(self):
        notice = self.catalog / 'accesskit/LICENSE-MIT'
        notice.write_text('corrupted')
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
            self.collect()
        notice.unlink()
        with self.assertRaisesRegex(ValueError, 'missing or not a regular file'):
            self.collect()
        self.assertEqual(list(self.notices.iterdir()), [])

    def test_t1280_catalog_schema_and_empty_group_are_rejected(self):
        self.mutate_catalog(lambda c: c.update(schema=2))
        with self.assertRaisesRegex(ValueError, 'unsupported'):
            self.collect()
        self.mutate_catalog(lambda c: c.update(schema=1))
        self.mutate_catalog(lambda c: c['crates']['accesskit-0.19.0'].update(notices=[]))
        with self.assertRaisesRegex(ValueError, 'no notices'):
            self.collect()

    def test_t1280_invalid_notice_paths_and_revision_are_rejected(self):
        for value in [None, '', '../foreign', '/tmp/foreign']:
            with self.subTest(value=value):
                self.mutate_catalog(lambda c: c['crates']['accesskit-0.19.0']['notices'][0].update(file=value))
                with self.assertRaises(ValueError):
                    self.collect()
        self.mutate_catalog(lambda c: c['crates']['accesskit-0.19.0']['notices'][0].update(file='accesskit/LICENSE-APACHE', revision='wrong'))
        with self.assertRaisesRegex(ValueError, 'provenance'):
            self.collect()

    def test_t1280_missing_catalog_and_oversized_metadata_are_rejected(self):
        path = self.catalog / 'manifest.json'
        path.unlink()
        with self.assertRaises(FileNotFoundError):
            self.collect()
        path.write_bytes(b' ' * (1024 * 1024 + 1))
        with self.assertRaisesRegex(ValueError, 'exceeds 1 MiB'):
            self.collect()
        path.write_text('not JSON')
        with self.assertRaises(json.JSONDecodeError):
            self.collect()

    def test_t1280_reserved_path_cannot_be_replaced(self):
        reserved = self.source / crate_supplements.OVERLAY
        reserved.symlink_to(self.root / 'absent')
        with self.assertRaisesRegex(ValueError, 'reserved'):
            self.collect()
        self.assertTrue(reserved.is_symlink())

    def test_t1280_existing_font_notices_remain_in_output(self):
        fonts = self.source / 'fonts'
        fonts.mkdir()
        (fonts / 'OFL.txt').write_text('font fixture notice')
        record = self.collect()
        native = next(row for row in record['notices'] if row['path'] == 'fonts/OFL.txt')
        self.assertNotIn('provenance', native)
        self.assertEqual(len(record['notices']), 3)

    def test_t1280_staging_is_reclaimed_after_success_error_and_cancellation(self):
        for failure in [None, OSError('injected'), KeyboardInterrupt()]:
            with self.subTest(failure=type(failure).__name__):
                stage = None
                try:
                    with crate_supplements.prepared_source(self.package) as (stage, provenance):
                        self.assertTrue(stage.is_dir())
                        self.assertEqual(len(provenance), 2)
                        if failure is not None:
                            raise failure
                except (OSError, KeyboardInterrupt):
                    self.assertIsNotNone(failure)
                self.assertFalse(stage.exists())
                self.assertTrue(self.source.is_dir())

    def test_t1280_partial_overlay_failure_reclaims_workspace(self):
        temporary = tempfile.TemporaryDirectory
        install = crate_supplements.install_notice
        def fail_second(source, pin, notice):
            if notice['file'].endswith('LICENSE-MIT'):
                raise OSError('disk full')
            return install(source, pin, notice)
        with patch.object(crate_supplements.tempfile, 'TemporaryDirectory',
                          side_effect=lambda **kw: temporary(dir=self.root, **kw)):
            with patch.object(crate_supplements, 'install_notice', side_effect=fail_second):
                with self.assertRaisesRegex(OSError, 'disk full'):
                    self.collect()
        self.assertEqual(list(self.root.glob('blent-crate-notices-*')), [])

    def test_t1280_copied_identity_is_rechecked_before_supplements(self):
        copytree = shutil.copytree
        staged = []
        def alter_copy(source, target, **kwargs):
            result = copytree(source, target, **kwargs)
            (Path(target) / 'Cargo.toml').write_text('changed during copy')
            staged.append(Path(target))
            return result
        with patch.object(crate_supplements.shutil, 'copytree', side_effect=alter_copy):
            with self.assertRaisesRegex(ValueError, 'manifest mismatch'):
                self.collect()
        self.assertEqual(len(staged), 1)
        self.assertFalse(staged[0].exists())
        self.assertEqual(list(self.notices.iterdir()), [])

    def test_t1280_catalog_pins_match_locked_crates_and_notice_bytes(self):
        locked = {(p['name'], p['version']): p for p in
                  tomllib.loads((REPO / 'Cargo.lock').read_text())['package']}
        catalog = json.loads((self.catalog / 'manifest.json').read_text())
        self.assertEqual(len(catalog['crates']), 18)
        for key, pin in catalog['crates'].items():
            with self.subTest(crate=key):
                self.assertEqual(key, f"{pin['name']}-{pin['version']}")
                self.assertEqual(pin['crate_sha256'], locked[pin['name'], pin['version']]['checksum'])
                self.assertEqual(pin['source'], locked[pin['name'], pin['version']]['source'])
                self.assertEqual(pin['revision'], pin['vcs_info']['git']['sha1'])
                self.assertTrue(pin['notices'])
                for notice in pin['notices']:
                    self.assertEqual(pin['revision'], notice['revision'])
                    self.assertEqual(hashlib.sha256((self.catalog / notice['file']).read_bytes()).hexdigest(), notice['sha256'])

    def test_t1280_donor_archive_is_pinned_to_the_lockfile(self):
        locked = {(p['name'], p['version']): p for p in
                  tomllib.loads((REPO / 'Cargo.lock').read_text())['package']}
        catalog = json.loads((self.catalog / 'manifest.json').read_text())
        for notice in catalog['crates']['atspi-proxies-0.9.0']['notices']:
            donor = notice['donor']
            self.assertEqual(donor['archive_sha256'], locked[donor['name'], donor['version']]['checksum'])
            self.assertEqual(donor['source'], locked[donor['name'], donor['version']]['source'])

    def test_t1280_symlinked_supplement_is_rejected(self):
        path = self.catalog / 'accesskit/LICENSE-MIT'
        path.unlink()
        path.symlink_to(self.catalog / 'accesskit/LICENSE-APACHE')
        with self.assertRaisesRegex(ValueError, 'symlink'):
            self.collect()
