"""T1338 bounded descriptor policy and exact configuration release."""
from pathlib import Path
import itertools
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AvailabilityTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='blent-t1338-')
        cls.addClassCleanup(cls.directory.cleanup)
        cls.binary = Path(cls.directory.name) / 'availability'
        fixture = ROOT / 'scripts/tests/fixtures/aoa'
        native = ROOT / 'host/accessory'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        '-fsanitize=address,undefined', '-I' + str(fixture), '-I' + str(native),
                        str(fixture / 'availability_fixture.c'), str(native / 'availability.c'),
                        str(native / 'contract.c'), '-o', str(cls.binary)], check=True)

    def classify(self, values):
        result = subprocess.run([self.binary, *map(str, values)], capture_output=True, check=True)
        return int(result.stdout)

    def test_t1338_only_exact_debugging_interface_reserves_device(self):
        for values in itertools.product([0, 254, 255], [0, 65, 66, 67, 255], [0, 1, 2, 255]):
            self.assertEqual(self.classify([*values, 0]), int(values == (255, 66, 1)), values)

    def test_t1338_unavailable_configuration_never_becomes_available(self):
        for status in [-2**31, *range(-12, 0), 1, 2**31 - 1]:
            self.assertEqual(self.classify([255, 66, 1, status]), 2)


if __name__ == '__main__':
    unittest.main()
