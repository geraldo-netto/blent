"""T879: selected device metadata must reject malformed values before USB I/O."""
from pathlib import Path
import random
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AccessorySelectionTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='blent-t879-selection-')
        cls.addClassCleanup(cls.directory.cleanup)
        cls.binary = Path(cls.directory.name) / 'selection'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        '-fsanitize=address,undefined', '-I' + str(ROOT / 'host/accessory'),
                        str(ROOT / 'scripts/tests/fixtures/aoa/selection.c'),
                        str(ROOT / 'host/accessory/contract.c'), '-o', str(cls.binary)],
                       check=True, timeout=30)

    def run_selection(self, values):
        return subprocess.run([self.binary, *values], capture_output=True, timeout=3)

    def test_t879_range_boundaries_round_trip(self):
        for values in [('1', '0', '65535'), ('127', '65535', '0'), ('7', '6353', '201')]:
            result = self.run_selection(values)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.decode().strip().split(), list(values))

    def test_t879_invalid_fields_reject_without_truncation(self):
        randomizer = random.Random(879)
        invalid = ['', '-1', '+1', ' 1', '1 ', '01', '1x', '1\n', '9' * 512]
        invalid += [str(randomizer.randrange(65536, 2**64)) for _ in range(32)]
        for index in range(3):
            for value in invalid:
                fields = ['7', '6353', '201']
                fields[index] = value
                self.assertEqual(self.run_selection(fields).returncode, 2, fields)

    def test_t879_address_rejects_zero_and_non_usb_range(self):
        for address in [0, 128, 255, 256, 65535]:
            self.assertEqual(self.run_selection([str(address), '6353', '201']).returncode, 2)


if __name__ == '__main__':
    unittest.main()
