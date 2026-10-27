"""T1278: exact installed files need matching, present attribution inputs."""
import importlib.util
import io
from contextlib import redirect_stdout, redirect_stderr
import json
import os
from pathlib import Path
import subprocess
import stat
from types import SimpleNamespace
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'verify-local-notices.py'
spec = importlib.util.spec_from_file_location('local_notices', SCRIPT)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class LocalNoticesTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        binary = self.root / 'usr/bin/blent'
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b'fixture executable')
        self.directory = self.root / checker.NOTICE_ROOT
        self.directory.mkdir(parents=True)
        notice = self.directory / 'LICENSE'
        notice.write_text('fixture notice')
        self.manifest = dict(schema='blent.local-notices.v1',
            bundle_files=[dict(path='usr/bin/blent', sha256=checker.digest(binary), group='project')],
            notice_files=[dict(path='LICENSE', sha256=checker.digest(notice))],
            groups=dict(project=dict(notices=['LICENSE'])), unresolved=[])

    def publish(self):
        (self.directory / 'manifest.json').write_text(json.dumps(self.manifest))

    def test_t1278_missing_manifest_fails_then_verified_recovery_passes(self):
        with self.assertRaises(FileNotFoundError):
            checker.verify(self.root)
        self.publish()
        self.assertEqual(checker.verify(self.root), (1, 1))
        result = subprocess.run([sys.executable, SCRIPT, self.root], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_t1278_missing_changed_and_uninventoried_files_fail(self):
        self.publish()
        (self.directory / 'LICENSE').unlink()
        with self.assertRaises(FileNotFoundError):
            checker.verify(self.root)
        (self.directory / 'LICENSE').write_text('changed')
        with self.assertRaisesRegex(ValueError, 'identity changed'):
            checker.verify(self.root)
        (self.directory / 'LICENSE').write_text('fixture notice')
        (self.root / 'usr/bin/unknown').write_text('unattributed')
        with self.assertRaisesRegex(ValueError, 'inventory differs'):
            checker.verify(self.root)

    def test_t1278_invalid_duplicate_and_unresolved_inputs_fail(self):
        for field, value in [('schema', 'old'), ('unresolved', ['missing license']), ('unresolved', None),
                             ('groups', {}), ('groups', []), ('notice_files', []),
                             ('notice_files', self.manifest['notice_files'] * 2)]:
            with self.subTest(field=field, value=value):
                before = self.manifest[field]
                self.manifest[field] = value
                self.publish()
                with self.assertRaises(ValueError):
                    checker.verify(self.root)
                self.manifest[field] = before
        self.manifest['groups']['project']['notices'] = ['absent']
        self.publish()
        with self.assertRaisesRegex(ValueError, 'missing notices'):
            checker.verify(self.root)

    def test_t1278_paths_cannot_escape_notice_owner(self):
        for name in ['', None, '/tmp/foreign', '../foreign', 'a/../../foreign']:
            with self.subTest(name=name), self.assertRaises(ValueError):
                checker.contained(self.root, name)
        (self.root / 'escape').symlink_to('/tmp')
        with self.assertRaises(ValueError):
            checker.contained(self.root, 'escape/foreign')

    def test_t1278_manifest_capacity_and_cli_fail_closed(self):
        path = self.directory / 'manifest.json'
        path.write_bytes(b' ' * (checker.MAX_MANIFEST + 1))
        with self.assertRaisesRegex(ValueError, 'capacity'):
            checker.verify(self.root)
        path.write_text('{}')
        result = subprocess.run([sys.executable, SCRIPT, self.root], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Notice verification failed', result.stderr)

    def test_t1278_project_dependency_attribution_cannot_disappear(self):
        self.manifest['groups']['project']['dependencies'] = ['missing-crate']
        self.publish()
        with self.assertRaisesRegex(ValueError, 'dependency is missing'):
            checker.verify(self.root)

    def test_t1278_special_files_are_rejected_before_reading(self):
        fifo = self.root / 'blocked-notice'
        os.mkfifo(fifo)
        with patch.object(Path, 'open', side_effect=AssertionError('attempted special-file open')), \
             patch.object(os, 'open', side_effect=AssertionError('attempted special-file open')):
            with self.assertRaisesRegex(ValueError, 'regular file'):
                checker.digest(fifo)
        with self.assertRaisesRegex(ValueError, 'regular file'):
            checker.digest(self.root)

    def test_t1278_descriptor_cleanup_on_rejection_and_stream_failure(self):
        path = self.root / 'usr/bin/blent'
        with patch.object(os, 'fstat', return_value=SimpleNamespace(st_mode=stat.S_IFIFO)):
            with patch.object(os, 'close', wraps=os.close) as closed:
                with self.assertRaisesRegex(ValueError, 'changed from a regular file'):
                    checker.digest(path)
                self.assertEqual(closed.call_count, 1)
        with patch.object(os, 'fdopen', side_effect=OSError('stream allocation failed')):
            with patch.object(os, 'close', wraps=os.close) as closed:
                with self.assertRaisesRegex(OSError, 'stream allocation failed'):
                    checker.digest(path)
                self.assertEqual(closed.call_count, 1)

    def test_t1278_manifest_must_be_regular_and_stay_inside_appdir(self):
        os.mkfifo(self.directory / 'manifest.json')
        with self.assertRaisesRegex(ValueError, 'regular file'):
            checker.verify(self.root)
        (self.directory / 'manifest.json').unlink()
        (self.directory / 'LICENSE').unlink()
        self.directory.rmdir()
        self.directory.symlink_to('/tmp')
        with self.assertRaisesRegex(ValueError, 'escapes its owner'):
            checker.verify(self.root)

    def test_t1278_cli_reports_success_and_missing_inputs(self):
        with patch.object(sys, 'argv', [str(SCRIPT), str(self.root)]):
            with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as exit:
                checker.main()
            self.assertEqual(exit.exception.code, 1)
            self.publish()
            output = io.StringIO()
            with redirect_stdout(output):
                checker.main()
            self.assertIn('Verified 1 installed files and 1 notice files', output.getvalue())

    def test_t1278_launcher_is_part_of_the_installed_identity(self):
        self.publish()
        launcher = self.root / 'AppRun'
        launcher.write_text('#!/bin/sh\n')
        with self.assertRaisesRegex(ValueError, 'inventory differs'):
            checker.verify(self.root)
        self.manifest['bundle_files'].append(dict(path='AppRun', sha256=checker.digest(launcher), group='project'))
        self.publish()
        self.assertEqual(checker.verify(self.root), (2, 1))

    def test_t1278_row_capacity_rejects_oversized_inventory(self):
        with self.assertRaisesRegex(ValueError, 'oversized inventory'):
            checker.checked_rows(self.root, [{}] * (checker.MAX_ROWS + 1))
