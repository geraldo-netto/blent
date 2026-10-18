"""T1079: progressing builds outlive monitoring intervals; cancellation joins owners."""
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import test_secure_connection_interop as interop


class ProgressingProcess:
    """Bounded synchronous process double; no native process or clock is allocated."""
    pid = 1079

    def __init__(self, output, interrupted=False, status=0):
        self.output = output
        self.interrupted = interrupted
        self.status = status
        self.joined = False
        self.waits = []

    def wait(self, timeout=None):
        self.waits.append(timeout)
        self.output.write('compiled next independent module\n')
        self.output.flush()
        if timeout is not None:
            raise subprocess.TimeoutExpired(['offline-build'], timeout)
        if self.interrupted:
            self.interrupted = False
            raise KeyboardInterrupt
        self.joined = True
        return self.status

    def poll(self):
        return self.status if self.joined else None


class SecureConnectionCommandTest(unittest.TestCase):
    def command(self, interrupted=False, status=0, missing_group=False):
        with tempfile.TemporaryDirectory(prefix='blent-t1079-') as temporary:
            logfile = Path(temporary) / 'progress.log'
            processes = []

            def spawn(arguments, **options):
                self.assertTrue(options['start_new_session'])
                process = ProgressingProcess(options['stdout'], interrupted, status)
                processes.append(process)
                return process

            with patch('test_secure_connection_interop.subprocess.Popen', side_effect=spawn), \
                    patch('test_secure_connection_interop.os.killpg') as kill:
                if missing_group:
                    kill.side_effect = ProcessLookupError
                runner = interop.SecureConnectionInteropTest()
                try:
                    runner.command(['offline-build'], {}, logfile)
                finally:
                    self.process = processes[0]
                    self.signals = kill.call_args_list
                    self.progress = logfile.read_text()
            self.assertTrue(self.process.output.closed)

    def test_t1079_progressing_build_has_no_elapsed_kill_deadline(self):
        self.command()
        self.assertTrue(self.process.joined)
        self.assertEqual([None], self.process.waits)
        self.assertEqual([], self.signals)
        self.assertIn('compiled next independent module', self.progress)

    def test_t1079_cancellation_stops_and_joins_original_process_group(self):
        with self.assertRaises(KeyboardInterrupt):
            self.command(interrupted=True)
        self.assertTrue(self.process.joined)
        self.assertEqual([None, None], self.process.waits)
        self.assertEqual([(1079, signal.SIGKILL)], [call.args for call in self.signals])
        self.assertTrue(self.process.output.closed)

    def test_t1079_already_gone_group_still_joins_original_child(self):
        with self.assertRaises(KeyboardInterrupt):
            self.command(interrupted=True, missing_group=True)
        self.assertTrue(self.process.joined)
        self.assertTrue(self.process.output.closed)

    def test_t1079_nonzero_exit_remains_failure_with_progress_log(self):
        with self.assertRaisesRegex(AssertionError, 'compiled next independent module'):
            self.command(status=23)
        self.assertTrue(self.process.joined)
        self.assertEqual([], self.signals)
        self.assertTrue(self.process.output.closed)


if __name__ == '__main__':
    unittest.main()
