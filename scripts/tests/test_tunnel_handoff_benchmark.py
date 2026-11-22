"""T1755: hosted record accounting compiles and runs through the normal tool suite."""
import os
from pathlib import Path
import tempfile
import unittest

import test_secure_connection_interop as interop

ROOT = Path(__file__).resolve().parents[2]


class TunnelHandoffBenchmarkTest(unittest.TestCase):
    def test_t1755_record_accounting_and_task_registration(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1755-fixture-') as temporary:
            logfile = Path(temporary) / 'tests.log'
            interop.SecureConnectionInteropTest().command([
                str(ROOT / 'android/gradlew'), '-p', str(ROOT / 'android'), '--offline', '--no-daemon',
                '-I', str(ROOT / 'scripts/benchmarks/android-tunnel/benchmark.init.gradle'),
                '-PblentTunnelOutput=' + str(Path(temporary) / 'unused.json'),
                ':app:testDebugUnitTest', '--tests', 'com.blent.connection.RecordMeterTest',
                '--tests', 'com.blent.connection.TunnelOutputTest',
                '--tests', 'com.blent.connection.LocalMeasurementTest',
                '--tests', 'com.blent.connection.ReadMeasurementTest',
                'help', '--task', 'tunnelHandoffBenchmark', 'tasks', '--all',
            ], dict(os.environ), logfile)
            self.assertIn('tunnelHandoffBenchmark', logfile.read_text())
            self.assertIn('localHandoffBenchmark', logfile.read_text())
            self.assertIn('tunnelReadBenchmark', logfile.read_text())


if __name__ == '__main__':
    unittest.main()
