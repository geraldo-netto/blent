"""T321/T1718: retain the independent numeric corpus against the current CLI."""
from pathlib import Path
import random
import subprocess
import tempfile
import unittest

from edid_tool import binary


class TimingTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = binary()

    def invoke(self, output, width, height, refresh, width_mm=310, height_mm=194):
        result = subprocess.run([str(self.binary), str(width), str(height), str(refresh),
                               str(output), str(width_mm), str(height_mm)],
                              capture_output=True, text=True)
        self.assertIn(result.returncode, (0, 1, 2), result.stderr)
        return result

    def test_t321_rejection_explains_low_clock_and_supported_refresh(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory, 'display.bin')
            for fps in [10, 24]:
                rejected = self.invoke(output, 640, 480, fps)
                self.assertNotEqual(rejected.returncode, 0)
                self.assertRegex(rejected.stderr, '10 MHz.*25 Hz')
            for width, height, fps in [(640, 480, 25), (640, 1212, 10), (90, 3960, 10)]:
                self.assertIsNotNone(self.check_mode(output, width, height, fps))

    def check_mode(self, output, width, height, fps):
        generated = self.invoke(output, width, height, fps)
        if generated.returncode:
            return
        result = output.read_bytes()
        self.assertEqual(len(result), 128)
        self.assertEqual(sum(result) % 256, 0)
        self.assertGreaterEqual(int.from_bytes(result[54:56], 'little'), 1000)
        self.assertLessEqual(result[113], fps)
        self.assertTrue(1 <= width <= 4095 and 1 <= height <= 4095)
        self.assertTrue(10 <= fps <= 90)
        return result

    def test_t321_invalid_and_out_of_bounds_numeric_corpus(self):
        rng = random.Random(321)
        cases = [(rng.randrange(-100, 5000), rng.randrange(-100, 5000), rng.randrange(-5, 100))
                 for _ in range(1024)]
        for value in [-2**63, -1, 0, 1, 4095, 4096, 2**32-1, 2**64]:
            cases.extend([(value, 480, 25), (640, value, 25), (640, 480, value)])
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory, 'display.bin')
            for values in cases:
                with self.subTest(values=values):
                    self.check_mode(output, *values)

    def test_t321_invalid_physical_dimensions_and_generator_cli(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory, 'display with spaces.bin')
            for value in [-1, 0, 4096, 2**64]:
                self.assertNotEqual(self.invoke(output, 640, 480, 25, width_mm=value).returncode, 0)
                self.assertNotEqual(self.invoke(output, 640, 480, 25, height_mm=value).returncode, 0)
            generated = self.invoke(output, 640, 480, 25)
            self.assertEqual(generated.returncode, 0, generated.stderr)
            expected = output.read_bytes()
            self.assertEqual(len(expected), 128)
            self.assertEqual(sum(expected) % 256, 0)
            rejected = self.invoke(output, 640, 480, 10)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn('10 MHz', rejected.stderr)
            self.assertEqual(output.read_bytes(), expected, 'rejected mode overwrote a valid EDID')


if __name__ == '__main__':
    unittest.main()
