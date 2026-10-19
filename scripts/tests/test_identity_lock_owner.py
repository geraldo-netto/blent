"""T1157: bounded native lock-fixture inputs and actual descriptor cleanup."""
import importlib.util
import io
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[2] / 'android/app/src/test/resources/identity-lock-owner.py'
SPEC = importlib.util.spec_from_file_location('identity_lock_owner', SOURCE)
OWNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OWNER)


class IdentityLockOwnerTest(unittest.TestCase):
    def test_t1157_count_path_integer_and_range_bounds(self):
        for count in list(range(5)) + list(range(6, 12)):
            with self.assertRaises(ValueError):
                OWNER.identity(['0'] * count)
        for value in ['-1', str(1 << 64), '9' * 21, 'NaN', '', '1.5']:
            with self.assertRaises(ValueError):
                OWNER.identity(['probe', 'file', 'marker', value, '0'])
        for size in [4097, 8192]:
            with self.assertRaises(ValueError):
                OWNER.identity(['probe', 'x' * size, 'marker', '0', '0'])
        for value in [0, 1, (1 << 64) - 1]:
            self.assertEqual((value, value), OWNER.identity(['probe', 'file', 'marker', str(value), str(value)]))

    def test_t1157_foreign_identity_rejects_and_original_close_returns(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1157-python-') as root:
            path = Path(root) / 'lock'
            path.touch(mode=0o600)
            original = path.stat()
            with patch.object(OWNER.os, 'close', wraps=os.close) as closed:
                with self.assertRaises(ValueError):
                    with OWNER.locked_file(path, (original.st_dev, original.st_ino + 1)):
                        self.fail('T1157 foreign identity yielded authority')
                self.assertEqual(1, closed.call_count)  # Wrapped close actually returned.

    def test_t1157_exact_ready_marker_and_one_byte_settlement(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1157-python-') as root:
            path = Path(root) / 'lock'
            path.touch(mode=0o600)
            original = path.stat()
            marker = Path(root) / 'ready'
            args = ['probe', str(path), str(marker), str(original.st_dev), str(original.st_ino)]
            for command, expected in [(b'R', 0), (b'', 2), (b'X', 2)]:
                stdin = type('Input', (), {'buffer': io.BytesIO(command)})()
                with patch.object(OWNER.sys, 'stdin', stdin):
                    self.assertEqual(expected, OWNER.main(args))
                self.assertEqual(b'H', marker.read_bytes())
                self.assertFalse(Path(str(marker) + '.pending').exists())


if __name__ == '__main__':
    unittest.main()
