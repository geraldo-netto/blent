"""T918: the offline benchmark task configures after AGP registers unit tasks."""
from pathlib import Path
import os
import tempfile
import unittest

import test_secure_connection_interop as interop

ROOT = Path(__file__).resolve().parents[2]


class ConnectionIdleBenchmarkTest(unittest.TestCase):
    def command(self, arguments):
        with tempfile.TemporaryDirectory(prefix='blent-t918-registration-') as temporary:
            output = Path(temporary)
            log = output / 'registration.log'
            interop.SecureConnectionInteropTest().command([
                str(ROOT / 'android/gradlew'), '-p', str(ROOT / 'android'), '--offline', '--no-daemon',
                '-I', str(ROOT / 'scripts/benchmarks/android-connection-idle/benchmark.init.gradle'),
                '-PblentIdleOutput=' + str(output / 'results.json'), *arguments,
            ], dict(os.environ), log)
            return log.read_text()

    def test_t918_task_registration_resolves_native_classpath_lazily(self):
        self.assertIn('connectionIdleBenchmark', self.command(['help', '--task', 'connectionIdleBenchmark']))

    def test_t918_hosted_counter_adapter_compiles_with_android_unit_sources(self):
        self.command([':app:compileDebugUnitTestKotlin'])


if __name__ == '__main__':
    unittest.main()
