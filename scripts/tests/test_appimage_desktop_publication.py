"""T1373: desktop registration preserves prior files on publication failure."""
from pathlib import Path
import tempfile
import unittest

from publication_fixture import destination, execute, staging


class AppImageDesktopPublicationTest(unittest.TestCase):
    def exercise(self, kind, writer='printf "new\\n"\n', overrides=''):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, retained = destination(root, 'applications/blent.desktop', kind)
            (root / 'write-desktop-entry.sh').write_text(writer)
            icon = root / 'usr/share/icons/hicolor/scalable/apps/blent.svg'
            icon.parent.mkdir(parents=True)
            icon.write_text('icon')
            result = execute(root, 'appimage', 'install_desktop',
                'install_gui_entry() { :; }\n' + overrides)
            self.assertEqual(staging(root), [])
            return result, retained.read_text()

    def test_t1373_directory_destinations_are_rejected(self):
        for kind in ['directory', 'symlink']:
            with self.subTest(kind=kind):
                result, contents = self.exercise(kind)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(contents, 'original\n')

    def test_t1373_success_publishes_complete_entry(self):
        result, contents = self.exercise('file')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(contents, 'new\n')

    def test_t1373_failed_renderer_preserves_original(self):
        result, contents = self.exercise('file', 'printf partial\nexit 42\n')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(contents, 'original\n')

    def test_t1373_failed_publication_preserves_original(self):
        for command in ['mv', 'mktemp']:
            with self.subTest(command=command):
                result, contents = self.exercise('file', overrides=f'{command}() {{ return 42; }}')
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(contents, 'original\n')
