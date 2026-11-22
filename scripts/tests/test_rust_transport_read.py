"""T1770: actual Rust transport/Rustls receive observations and exact cleanup."""
import json
import os
from pathlib import Path
import tempfile
import unittest

import test_secure_connection_interop as interop

ROOT = Path(__file__).resolve().parents[2]


class RustTransportReadTest(unittest.TestCase):
    def test_t1770_actual_rust_peer_and_pipelined_receive(self):
        environment = dict(os.environ, CARGO_NET_OFFLINE='true')
        target = Path(environment.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve()
        with tempfile.TemporaryDirectory(prefix='blent-t1770-tests-') as temporary:
            directory = Path(temporary)
            command = interop.SecureConnectionInteropTest().command
            command(['cargo', 'test', '--offline', '--locked', '-p', 'blent',
                     '--features', 'accessory-transport', '--example', 'tunnel_read_fixture',
                     '--target-dir', str(target)], environment, directory / 'rust-tests.log')
            command(['cargo', 'build', '--offline', '--locked', '-p', 'blent',
                     '--features', 'accessory-transport', '--example', 'tunnel_read_fixture',
                     '--target-dir', str(target)], environment, directory / 'rust-build.log')
            output = directory / 'observations.json'
            command([str(ROOT / 'android/gradlew'), '-p', str(ROOT / 'android'), '--offline', '--no-daemon',
                     '-I', str(ROOT / 'scripts/benchmarks/android-tunnel/benchmark.init.gradle'),
                     '-PblentRustReadFixture=' + str(target / 'debug/examples/tunnel_read_fixture'),
                     '-PblentRustReadOutput=' + str(output), ':app:testDebugUnitTest',
                     '--tests', 'com.blent.connection.RustTransportReadTest',
                     '--tests', 'com.blent.connection.RecordMeterTest',
                     '--tests', 'com.blent.connection.RecordInputMeterTest',
                     '--tests', 'com.blent.connection.RustProcessTest'], environment, directory / 'android.log')
            self.assertEqual(len(json.loads(output.read_text())), 16)


if __name__ == '__main__':
    unittest.main()
