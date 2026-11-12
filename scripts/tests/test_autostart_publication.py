"""T1370: exact autostart publication and owned staging cleanup."""
from pathlib import Path
import tempfile
import unittest

from publication_fixture import destination, execute, staging


class AutostartPublicationTest(unittest.TestCase):
    def exercise(self, adapter, kind, overrides=''):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entry, retained = destination(root, 'autostart/blent.desktop', kind)
            (root / 'blent-service-autostart.desktop').write_text('new\n')
            (root / 'write-desktop-entry.sh').write_text('printf "new\\n"\n')
            command = ('install_autostart_entry printf "new\\n"' if adapter == 'source'
                       else 'update_autostart')
            result = execute(root, adapter, command, overrides)
            self.assertEqual(staging(root), [])
            return result, retained.read_text(), entry.is_symlink()

    def test_t1370_directory_destinations_are_rejected(self):
        for adapter in ['source', 'appimage']:
            for kind in ['directory', 'symlink']:
                with self.subTest(adapter=adapter, kind=kind):
                    result, contents, link = self.exercise(adapter, kind)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(contents, 'original\n')
                    self.assertEqual(link, kind == 'symlink')

    def test_t1370_success_replaces_the_exact_file(self):
        for adapter in ['source', 'appimage']:
            result, contents, _ = self.exercise(adapter, 'file')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(contents, 'new\n')

    def test_t1370_unmanaged_autostart_keeps_its_preference(self):
        result, contents, _ = self.exercise('appimage', 'file', 'systemctl() { return 1; }')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(contents, 'new\n')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = execute(root, 'appimage', 'update_autostart', 'systemctl() { return 1; }')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse((root / 'autostart').exists())

    def test_t1370_failed_publication_preserves_file(self):
        for adapter in ['source', 'appimage']:
            for override in ['mv() { return 42; }', 'mktemp() { return 42; }']:
                with self.subTest(adapter=adapter, override=override):
                    result, contents, _ = self.exercise(adapter, 'file', override)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(contents, 'original\n')
