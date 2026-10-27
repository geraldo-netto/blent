"""T1302: exercise actual Gradle Test configuration, including invalid budgets."""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AndroidTestResourcesTest(unittest.TestCase):
    def probe(self, properties=()):
        with tempfile.TemporaryDirectory(prefix='blent-t1302-') as directory:
            report = Path(directory) / 'workers.tsv'
            command = [str(ROOT / 'android/gradlew'), '-p', str(ROOT / 'android'),
                       '--offline', '--no-daemon', '--console=plain', '-q',
                       '-I', str(ROOT / 'scripts/tests/fixtures/android-test-resources/probe.init.gradle.kts'),
                       f'-PblentTestProbeOutput={report}', *properties, 't1302Probe']
            result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            rows = report.read_text().splitlines() if report.exists() else []
            return result, [row.split('\t') for row in rows]

    def check_budget(self, properties, heap, batch):
        result, rows = self.probe(properties)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        names = {row[0] for row in rows}
        self.assertTrue({'testDebugUnitTest', 'tlsInteropTest'} <= names)
        for name, actual_heap, actual_batch, declared in rows:
            self.assertEqual((actual_heap, actual_batch, declared),
                             (f'{heap}m', str(batch), str(heap)), name)

    def test_t1302_defaults_fund_every_test_task(self):
        self.check_budget((), 2048, 40)

    def test_t1302_explicit_budget_boundaries(self):
        for heap, batch in [(128, 1), (16384, 1000)]:
            with self.subTest(heap=heap, batch=batch):
                self.check_budget((f'-PblentTestHeapMiB={heap}', f'-PblentTestForkEvery={batch}'), heap, batch)

    def test_t1302_invalid_budgets_fail_before_workers(self):
        for name, values in [('blentTestHeapMiB', ['127', '16385', '2147483648', 'bad']),
                             ('blentTestForkEvery', ['0', '1001', '-1', 'bad'])]:
            for value in values:
                with self.subTest(name=name, value=value):
                    result, rows = self.probe((f'-P{name}={value}',))
                    self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertIn(name, result.stderr)
                    self.assertFalse(rows)


if __name__ == '__main__':
    unittest.main()
