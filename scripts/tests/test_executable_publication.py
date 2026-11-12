"""T1372: AppImage launch paths name exact, complete executable files."""
from pathlib import Path
import tempfile
import unittest

from publication_fixture import destination, execute, staging

TARGETS = {'image': 'Blent.AppImage', 'entry': 'blent', 'gui': 'blent-gui'}
COMMANDS = {'image': 'install_image', 'entry': 'install_entry', 'gui': 'install_gui_entry'}


class ExecutablePublicationTest(unittest.TestCase):
    def exercise(self, role, kind, overrides=''):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entry, retained = destination(root, TARGETS[role], kind)
            (root / 'image').write_text('new image\n')
            (root / '.local/bin').mkdir(parents=True)
            result = execute(root, 'appimage', COMMANDS[role], overrides)
            self.assertEqual(staging(root), [])
            return result, retained.read_text(), entry.stat().st_mode & 0o777

    def test_t1372_directory_destinations_are_rejected(self):
        for role in TARGETS:
            for kind in ['directory', 'symlink']:
                with self.subTest(role=role, kind=kind):
                    result, contents, _ = self.exercise(role, kind)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(contents, 'original\n')

    def test_t1372_success_publishes_executable(self):
        for role in TARGETS:
            result, contents, mode = self.exercise(role, 'file')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotEqual(contents, 'original\n')
            self.assertEqual(mode, 0o755)

    def test_t1372_failures_preserve_original_and_remove_staging(self):
        for role in TARGETS:
            for command in ['mv', 'chmod', 'mktemp']:
                with self.subTest(role=role, command=command):
                    result, contents, _ = self.exercise(role, 'file', f'{command}() {{ return 42; }}')
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(contents, 'original\n')

    def test_t1372_failed_content_generation_preserves_original(self):
        for role in TARGETS:
            with self.subTest(role=role):
                result, contents, _ = self.exercise(role, 'file',
                    'cp() { return 42; }; cat() { return 42; }')
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(contents, 'original\n')
