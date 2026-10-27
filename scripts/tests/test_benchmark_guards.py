"""T1287: benchmark assertions contain required effects and cannot be removed."""
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class BenchmarkGuardsTest(unittest.TestCase):
    def test_t1287_required_assertions_cannot_be_disabled(self):
        for name in ('frame-retirement', 'shared-damage', 'idle_writer'):
            with self.subTest(benchmark=name):
                result = subprocess.run(
                    ['cc', '-E', '-DNDEBUG', '-Ihost/evdi',
                     f'scripts/benchmarks/{name}.c'], cwd=ROOT,
                    capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('require assertions', result.stderr)
