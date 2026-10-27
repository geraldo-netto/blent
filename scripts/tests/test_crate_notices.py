"""T1279: preserve nested notices and reject absent declared attribution."""
import hashlib
import os
import stat
import tarfile
from types import SimpleNamespace
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'packaging/appimage'))
import sources
import crate_notices


class CrateNoticesTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='blent-notices-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / 'crate'
        self.source.mkdir()
        (self.source / 'Cargo.toml').write_text('[package]\n')
        self.archives = self.root / 'archives'
        self.archives.mkdir()
        self.notices = self.root / 'notices'
        self.notices.mkdir()
        self.package = dict(name='fixture', version='1.0.0', source='registry',
                            license='MIT', license_file=None,
                            manifest_path=str(self.source / 'Cargo.toml'))

    def write(self, name, content='copyright and permission fixture'):
        path = self.source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def collect(self):
        return sources.copy_crate(self.package, self.archives, self.notices)

    def test_t1279_nested_fonts_unlicense_and_notice_directory_are_preserved(self):
        names = ['fonts/OFL.txt', 'fonts/UFL.txt', 'fonts/Hack-Regular.txt',
                 'fonts/emoji-icon-font-mit-license.txt', 'UNLICENSE',
                 'vendor/COPYING.LESSER', 'LICENSES/component/terms.txt']
        for name in names:
            self.write(name)
        self.write('fonts/Hack-Regular.ttf', 'not a notice')
        self.collect()
        root = self.notices / 'fixture-1.0.0'
        actual = sorted(str(path.relative_to(root)) for path in root.rglob('*') if path.is_file())
        self.assertEqual(actual, sorted(names))
        for name in names:
            self.assertEqual((root / name).read_bytes(), (self.source / name).read_bytes())

    def test_t1279_declared_license_file_uses_metadata_and_preserves_path(self):
        self.write('legal/terms.txt')
        self.package['license_file'] = 'legal/terms.txt'
        self.collect()
        self.assertTrue((self.notices / 'fixture-1.0.0/legal/terms.txt').is_file())

    def test_t1279_missing_notices_abort_instead_of_publishing_empty_directory(self):
        with self.assertRaisesRegex(ValueError, 'notice'):
            self.collect()
        self.assertEqual(list(self.notices.iterdir()), [])
        self.assertEqual(list(self.archives.iterdir()), [])

    def test_t1279_missing_declared_file_is_not_hidden_by_another_license(self):
        self.write('LICENSE')
        self.package['license_file'] = 'absent.txt'
        with self.assertRaisesRegex(ValueError, 'notice'):
            self.collect()
        self.assertEqual(list(self.notices.iterdir()), [])

    def test_t1279_provenance_binds_notices_to_the_source_archive(self):
        self.write('LICENSE')
        self.write('nested/NOTICE')
        record = self.collect()
        self.assertIsNone(record['license_file'])
        self.assertEqual([r['path'] for r in record['notices']], ['LICENSE', 'nested/NOTICE'])
        archive_path = self.archives / 'fixture-1.0.0.tar.gz'
        with tarfile.open(archive_path) as archive:
            for row in record['notices']:
                with archive.extractfile('fixture-1.0.0/' + row['path']) as stream:
                    content = stream.read()
                self.assertEqual(hashlib.sha256(content).hexdigest(), row['sha256'])

    def test_t1279_invalid_declared_paths_never_read_outside_the_crate(self):
        self.write('LICENSE')
        for value in ['', 0, [], '../foreign', '/tmp/foreign', 'legal/../../foreign']:
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, 'notice'):
                crate_notices.copy_crate_notices(self.source, self.root / 'output', value)
        self.assertFalse((self.root / 'output').exists())

    def test_t1279_symlink_and_special_notice_inputs_are_rejected(self):
        self.write('LICENSE')
        (self.source / 'NOTICE').symlink_to(self.source / 'LICENSE')
        with self.assertRaisesRegex(ValueError, 'symlink'):
            self.collect()
        (self.source / 'NOTICE').unlink()
        os.mkfifo(self.source / 'NOTICE')
        with self.assertRaisesRegex(ValueError, 'regular file'):
            self.collect()
        self.assertEqual(list(self.notices.iterdir()), [])

    def test_t1279_declared_path_cannot_cross_symlinked_parent(self):
        external = self.root / 'external'
        external.mkdir()
        (external / 'terms.txt').write_text('foreign notice')
        (self.source / 'legal').symlink_to(external, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'symlink'):
            crate_notices.copy_crate_notices(self.source, self.root / 'output', 'legal/terms.txt')

    def test_t1279_empty_notices_and_copy_failure_clean_only_owned_output(self):
        self.write('LICENSE', '   \n\t')
        with self.assertRaisesRegex(ValueError, 'no text'):
            self.collect()
        self.assertEqual(list(self.notices.iterdir()), [])
        self.write('LICENSE')
        with patch.object(crate_notices, 'copy_notice', side_effect=OSError('disk full')):
            with self.assertRaisesRegex(OSError, 'disk full'):
                self.collect()
        self.assertEqual(list(self.notices.iterdir()), [])
        destination = self.notices / 'fixture-1.0.0'
        destination.mkdir()
        (destination / 'owned-by-other').write_text('keep')
        with self.assertRaises(FileExistsError):
            self.collect()
        self.assertEqual((destination / 'owned-by-other').read_text(), 'keep')

    def test_t1279_existing_archive_and_invalid_name_are_not_overwritten(self):
        self.write('LICENSE')
        target = self.archives / 'fixture-1.0.0.tar.gz'
        target.write_bytes(b'owned by other')
        with self.assertRaises(FileExistsError):
            self.collect()
        self.assertEqual(target.read_bytes(), b'owned by other')
        self.assertEqual(list(self.notices.iterdir()), [])
        self.package['name'] = '../escape'
        with self.assertRaisesRegex(ValueError, 'invalid crate output name'):
            self.collect()

    def test_t1279_cancellation_removes_partially_copied_notices(self):
        self.write('LICENSE')
        self.write('NOTICE')
        original = crate_notices.copy_notice
        def interrupt(source, target):
            if source.name == 'NOTICE':
                raise KeyboardInterrupt()
            return original(source, target)
        with patch.object(crate_notices, 'copy_notice', side_effect=interrupt):
            with self.assertRaises(KeyboardInterrupt):
                self.collect()
        self.assertEqual(list(self.notices.iterdir()), [])
        self.assertEqual(list(self.archives.iterdir()), [])

    def test_t1279_changed_source_notice_aborts_package_record(self):
        self.write('LICENSE')
        original = sources.copy_crate_notices
        def mutate(*args):
            result = original(*args)
            self.write('LICENSE', 'changed after notice copy')
            return result
        with patch.object(sources, 'copy_crate_notices', side_effect=mutate):
            with self.assertRaisesRegex(ValueError, 'changed while archiving'):
                self.collect()

    def test_t1279_nonregular_archive_member_is_not_accepted_as_provenance(self):
        archive_path = self.archives / 'fixture.tar.gz'
        with tarfile.open(archive_path, 'w:gz') as archive:
            member = tarfile.TarInfo('fixture/LICENSE')
            member.type = tarfile.SYMTYPE
            member.linkname = '/foreign'
            archive.addfile(member)
        with self.assertRaisesRegex(ValueError, 'not a regular file'):
            crate_notices.verify_archive_notices(archive_path, 'fixture', [dict(path='LICENSE', sha256='0'*64)])

    def test_t1279_regular_reader_closes_rejected_or_failed_descriptors(self):
        path = self.write('LICENSE')
        with patch.object(os, 'fstat', return_value=SimpleNamespace(st_mode=stat.S_IFIFO)):
            with patch.object(os, 'close', wraps=os.close) as closed:
                with self.assertRaisesRegex(ValueError, 'changed from a regular file'):
                    with crate_notices.regular_reader(path):
                        self.fail('nonregular input admitted')
                self.assertEqual(closed.call_count, 1)
        with patch.object(os, 'fdopen', side_effect=OSError('stream failure')):
            with patch.object(os, 'close', wraps=os.close) as closed:
                with self.assertRaisesRegex(OSError, 'stream failure'):
                    with crate_notices.regular_reader(path):
                        self.fail('failed stream admitted')
                self.assertEqual(closed.call_count, 1)

    def test_t1279_missing_source_and_direct_special_reader_fail(self):
        with self.assertRaisesRegex(ValueError, 'not a directory'):
            crate_notices.notice_paths(self.root / 'absent', None)
        with self.assertRaisesRegex(ValueError, 'regular file'):
            with crate_notices.regular_reader(self.source):
                self.fail('directory admitted')

    def test_t1279_name_matching_preserves_notice_variants(self):
        for name in ['LICENSE', 'Licence.md', 'COPYING3', 'Copyright', 'NOTICE',
                     'UNLICENSE', 'license-data', 'OFL.txt', 'UFL.txt',
                     'Hack-Regular.txt', 'font-mit-license.txt', 'font-licence.txt']:
            with self.subTest(name=name):
                self.assertTrue(crate_notices.notice_name(name))
        for name in ['', 'README.md', 'font.ttf', 'ordinary.txt']:
            self.assertFalse(crate_notices.notice_name(name))

    def test_t1279_stream_boundaries_preserve_bytes_and_digest(self):
        for size in [1, 65535, 65536, 65537, 131073]:
            with self.subTest(size=size):
                content = b' ' * (size - 1) + b'x'
                source = self.write('LICENSE')
                source.write_bytes(content)
                target = self.root / str(size)
                checksum = crate_notices.copy_notice(source, target)
                self.assertEqual(target.read_bytes(), content)
                self.assertEqual(checksum, hashlib.sha256(content).hexdigest())
