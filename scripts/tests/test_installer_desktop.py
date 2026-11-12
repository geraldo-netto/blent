"""T1362: failed launcher publication preserves the installed entry."""
from pathlib import Path
import tempfile
import unittest

from shell_fixture import run

ROOT = Path(__file__).resolve().parents[2]
SOURCE = (ROOT / 'scripts/install.sh').read_text().removesuffix('main "$@"\n')


class DesktopInstallTest(unittest.TestCase):
    def exercise(self, writer, publication='', directory=False):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entry = root / 'blent.desktop'
            if directory:
                entry.mkdir()
            retained = entry / 'retained' if directory else entry
            retained.write_text('original launcher\n')
            (root / 'write-desktop-entry.sh').write_text(writer)
            script = SOURCE + '\nSCRIPT_DIR=$1\nAPP_DIR=$1\nBIN_DIR=$1\n' + publication + '\ninstall_desktop_entry\n'
            result = run(script, [root], cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(sorted(p.name for p in root.iterdir()), ['blent.desktop', 'write-desktop-entry.sh'])
            if directory:
                self.assertEqual([p.name for p in entry.iterdir()], ['retained'])
            return result, retained.read_text()

    def test_t1362_writer_failure_preserves_original_and_propagates(self):
        result, contents = self.exercise('printf "partial launcher"\nexit 42\n')
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(contents, 'original launcher\n')
        self.assertNotIn('Desktop entry installed', result.stdout)

    def test_t1362_publication_failure_preserves_original_and_cleans_staging(self):
        result, contents = self.exercise('printf "new launcher\\n"\n', 'mv() { return 1; }')
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(contents, 'original launcher\n')
        self.assertNotIn('Desktop entry installed', result.stdout)

    def test_t1362_success_replaces_original_once(self):
        result, contents = self.exercise('printf "new launcher\\n"\n')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(contents, 'new launcher\n')
        self.assertIn('Desktop entry installed', result.stdout)

    def test_t1362_staging_failure_preserves_original(self):
        result, contents = self.exercise('printf "new launcher\\n"\n', 'mktemp() { return 1; }')
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(contents, 'original launcher\n')

    def test_t1362_directory_destination_is_rejected_without_moving_into_it(self):
        result, contents = self.exercise('printf "new launcher\\n"\n', directory=True)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(contents, 'original launcher\n')


if __name__ == '__main__':
    unittest.main()
