"""T846: required offline Rust/Android TLS interoperability, with no conditional skips."""
from pathlib import Path
import os
import signal
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class SecureConnectionInteropTest(unittest.TestCase):
    def command(self, arguments, environment, logfile):
        with logfile.open('w') as output:
            process = subprocess.Popen(arguments, cwd=ROOT, env=environment,
                                       stdout=output, stderr=subprocess.STDOUT,
                                       start_new_session=True)
            try:
                # T1079: builds have no elapsed kill cap; the caller monitors the log.
                status = process.wait()
            except BaseException:
                self.cancel_command(process)
                raise
        self.assertEqual(status, 0, logfile.read_text()[-12000:])

    def cancel_command(self, process):
        """Keep the original child/log owned until cancellation has joined it."""
        try:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass  # Original group exited between observation and signal.
        finally:
            process.wait()

    def test_t846_native_rust_android_tls_interoperability(self):
        environment = dict(os.environ, CARGO_NET_OFFLINE='true')
        target = Path(environment.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve()
        with tempfile.TemporaryDirectory(prefix='blent-t846-interop-build-') as temporary:
            logs = Path(temporary)
            self.command(['cargo', 'build', '--offline', '--locked', '-p', 'blent',
                          '--features', 'accessory-transport', '--example', 'secure_connection_fixture',
                          '--target-dir', str(target)], environment, logs / 'rust.log')
            fixture = target / 'debug/examples/secure_connection_fixture'
            self.assertTrue(fixture.is_file())
            self.command([str(ROOT / 'android/gradlew'), '-p', str(ROOT / 'android'),
                          '--offline', '--no-daemon', '-PblentTlsFixture=' + str(fixture),
                          ':app:tlsInteropTest'], environment, logs / 'android.log')

    def test_t846_unanswered_probe_approval_exits_after_deadline(self):
        environment = dict(os.environ, CARGO_NET_OFFLINE='true')
        target = Path(environment.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve()
        with tempfile.TemporaryDirectory(prefix='blent-t846-approval-') as temporary:
            self.command(['cargo', 'build', '--offline', '--locked', '-p', 'blent',
                          '--features', 'accessory-transport', '--example', 'accessory_probe',
                          '--target-dir', str(target)], environment, Path(temporary) / 'build.log')
            process = subprocess.Popen([str(target / 'debug/examples/accessory_probe'), '--approval-check'],
                                       stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                       stderr=subprocess.PIPE, start_new_session=True)
            try:
                # Keep stdin open and silent: communicate() would close it and mask the bug.
                try:
                    status = process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    self.fail('T846 approval timeout left the probe process stuck')
                self.assertNotEqual(status, 0)
                self.assertIn(b'deadline has elapsed', process.stderr.read())
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
                process.stdin.close()
                process.stdout.close()
                process.stderr.close()


if __name__ == '__main__':
    unittest.main()
