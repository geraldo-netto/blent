"""T1237: original USB epoch and queued bytes cannot cross a reopened link."""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AccessoryReconnectTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='blent-t1237-reconnect-')
        cls.addClassCleanup(cls.directory.cleanup)
        cls.binary = Path(cls.directory.name) / 'reconnect'
        fixture = ROOT / 'scripts/tests/fixtures/aoa'
        adapter = ROOT / 'host/accessory'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        '-fsanitize=address,undefined', '-I' + str(fixture), '-I' + str(adapter),
                        str(fixture / 'reconnect.c'), str(adapter / 'accessory.c'),
                        str(adapter / 'contract.c'), '-o', str(cls.binary)], check=True)

    def run_case(self, mode, *values):
        return subprocess.run([self.binary, str(mode), *map(str, values)], capture_output=True, text=True)

    def test_t1237_reopening_delivers_current_approval_marker(self):
        for mode in (0, 10):
            with self.subTest(mode=mode):
                result = self.run_case(mode)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_t1237_same_original_device_cannot_establish_reset_completion(self):
        result = self.run_case(1)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('fresh USB device', result.stderr)

    def test_t1237_failed_reset_cannot_admit_or_read_queued_data(self):
        result = self.run_case(2)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('USB reset failed', result.stderr)

    def test_t1237_native_reenumeration_results_still_require_fresh_identity(self):
        for mode in (3, 4, 5):
            with self.subTest(mode=mode):
                result = self.run_case(mode)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_t1237_absent_and_transient_replacement_preserve_original_reference(self):
        for mode in (6, 7):
            with self.subTest(mode=mode):
                result = self.run_case(mode)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_t1237_stale_selection_rejects_before_reset(self):
        result = self.run_case(8)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('selected device changed', result.stderr)

    def test_t1237_foreign_accessory_claim_prevents_physical_reset(self):
        result = self.run_case(11)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('accessory interface busy', result.stderr)

    def test_t1237_bounded_native_status_domain_rejects_unknown_results(self):
        statuses = [-2**31, -99, *range(-12, 3), 2**31 - 1]
        for status in statuses:
            with self.subTest(status=status):
                result = self.run_case(9, status)
                expected = 0 if status in {0, -4, -5} else 2
                self.assertEqual(result.returncode, expected, result.stderr)


if __name__ == '__main__':
    unittest.main()
