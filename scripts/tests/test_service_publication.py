"""T1371: service publication cannot redirect or leave failed staging."""
from pathlib import Path
import tempfile
import unittest

from publication_fixture import destination, execute, staging


class ServicePublicationTest(unittest.TestCase):
    def exercise(self, adapter, kind, writer='printf "new\\n"\n', overrides=''):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, retained = destination(root, 'systemd/user/blent.service', kind)
            (root / 'write-systemd-service.sh').write_text(writer)
            command = ('write_installed_user_service' if adapter == 'source' else 'install_service')
            result = execute(root, adapter, command, overrides)
            self.assertEqual(staging(root), [])
            return result, retained.read_text()

    def test_t1371_directory_destinations_are_rejected(self):
        for adapter in ['source', 'appimage']:
            for kind in ['directory', 'symlink']:
                with self.subTest(adapter=adapter, kind=kind):
                    result, contents = self.exercise(adapter, kind)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(contents, 'original\n')

    def test_t1371_failed_renderer_preserves_original(self):
        for adapter in ['source', 'appimage']:
            with self.subTest(adapter=adapter):
                result, contents = self.exercise(adapter, 'file', 'printf partial\nexit 42\n')
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(contents, 'original\n')

    def test_t1371_success_publishes_complete_service(self):
        for adapter in ['source', 'appimage']:
            result, contents = self.exercise(adapter, 'file')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(contents.endswith('new\n'))

    def test_t1371_failed_publication_preserves_original(self):
        for adapter in ['source', 'appimage']:
            for override in ['mv() { return 42; }', 'mktemp() { return 42; }']:
                with self.subTest(adapter=adapter, override=override):
                    result, contents = self.exercise(adapter, 'file', overrides=override)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(contents, 'original\n')
