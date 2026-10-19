"""T1108: progressing benchmark tools retain original child/pipe ownership."""
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'benchmarks'))
import profile_usb_pipeline as pipeline
IDLE = pipeline.module('idle-writer')
import profile_process as owned


class Progressing:
    """Finite original-process double; no native PID, time or allocation is borrowed."""
    def __init__(self, arguments, options, ordinal):
        self.args, self.options = arguments, options
        self.pid = 11080 + ordinal
        self.returncode = None
        self.stdin = io.BytesIO() if options.get('stdin') == subprocess.PIPE else None
        self.stdout = io.BytesIO() if options.get('stdout') == subprocess.PIPE else None
        self.stderr = io.BytesIO() if options.get('stderr') == subprocess.PIPE else None
        self.requested = False
        self.joined = False
        self.waits = []
        self.units = []
        self.fail_join_report = False
        self.communication_failure = None
        self.interruptions = 0

    def wait(self, timeout=None):
        self.waits.append(timeout)
        self.units.append('completed another compilation/cleanup unit')
        if timeout is not None:
            raise subprocess.TimeoutExpired(self.args, timeout)
        if self.interruptions:
            self.interruptions -= 1
            raise KeyboardInterrupt
        self.joined = True
        self.returncode = -9 if self.requested else 0
        if self.fail_join_report:
            self.fail_join_report = False
            raise RuntimeError('post-join report failed')
        return self.returncode

    def communicate(self, input=None, timeout=None):
        if self.communication_failure is not None:
            failure = self.communication_failure
            self.communication_failure = None
            raise failure
        self.wait(timeout)
        output = json.dumps({'libx264': {'encoder': 'libx264', 'options': []}})
        if not self.options.get('text'):
            output = output.encode()
        return output, None

    def poll(self):
        return self.returncode

    def kill(self):
        self.requested = True

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.wait()


class IdleWriterProcessTest(unittest.TestCase):
    def setUp(self):
        self.originals = []
        self.fail_second = False
        self.first_report_fails = False
        self.communication_failure = None
        self.interruptions = 0
        self.addCleanup(patch.stopall)
        patch('subprocess.Popen', side_effect=self.spawn).start()
        patch('os.killpg', side_effect=self.stop_group).start()

    def spawn(self, arguments, **options):
        if self.fail_second and self.originals:
            raise OSError('encoder admission failed')
        original = Progressing(arguments, options, len(self.originals))
        original.fail_join_report = self.first_report_fails and not self.originals
        original.communication_failure = self.communication_failure
        original.interruptions = self.interruptions
        self.originals.append(original)
        return original

    def stop_group(self, identity, requested):
        self.assertEqual(requested, signal.SIGKILL)
        original = next(item for item in self.originals if item.pid == identity)
        original.requested = True

    def settled(self):
        self.assertTrue(self.originals)
        for original in self.originals:
            self.assertTrue(original.joined)
            self.assertTrue(original.units)
            self.assertTrue(all(wait is None for wait in original.waits))
            self.assertTrue(original.options.get('start_new_session'))

    def test_t1108_progressing_compiler_has_no_elapsed_kill_cap(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1108-build-') as temporary:
            IDLE.build(Path(temporary) / 'candidate', 200)
        self.settled()

    def test_t1108_policy_build_is_offline_locked_and_has_no_elapsed_kill_cap(self):
        self.assertIn('libx264', pipeline.policies(60, 20000, 18))
        self.settled()
        self.assertIn('--offline', self.originals[0].args)
        self.assertIn('--locked', self.originals[0].args)

    def test_t1108_partial_encoder_start_still_joins_and_closes_original_producer(self):
        self.fail_second = True
        with tempfile.TemporaryDirectory(prefix='blent-t1108-start-') as temporary:
            with self.assertRaisesRegex(OSError, 'encoder admission failed'):
                IDLE.execute(Path('/owned-writer'), ['owned-encoder'], 60, False, Path(temporary))
        self.settled()
        self.assertTrue(self.originals[0].stdout.closed)

    def test_t1108_failed_producer_report_cannot_skip_original_encoder_cleanup(self):
        self.first_report_fails = True
        with tempfile.TemporaryDirectory(prefix='blent-t1108-report-') as temporary:
            with patch.object(IDLE, 'collect', side_effect=ValueError('sample rejected')):
                with self.assertRaisesRegex(RuntimeError, 'post-join report failed'):
                    IDLE.execute(Path('/owned-writer'), ['owned-encoder'], 60, False, Path(temporary))
        self.settled()
        self.assertTrue(all(original.stdout.closed for original in self.originals))

    def test_t1108_interrupted_capture_stops_original_group_before_closing_pipes(self):
        self.communication_failure = KeyboardInterrupt()
        with self.assertRaises(KeyboardInterrupt):
            owned.captured(['owned-analysis'], input=b'bounded record')
        self.settled()
        original = self.originals[0]
        self.assertTrue(original.requested)
        self.assertTrue(original.stdin.closed)
        self.assertTrue(original.stdout.closed)

    def test_t1108_failed_capture_still_joins_original_and_closes_owned_pipes(self):
        self.communication_failure = OSError('original output failed')
        with self.assertRaisesRegex(OSError, 'original output failed'):
            owned.captured(['owned-analysis'], input=b'bounded record')
        self.settled()
        self.assertTrue(self.originals[0].requested)
        self.assertTrue(self.originals[0].stdout.closed)

    def test_t1108_repeated_interruptions_cannot_abandon_original_retirement(self):
        self.interruptions = 2
        with self.assertRaises(KeyboardInterrupt):
            owned.captured(['owned-analysis'], input=b'bounded record')
        self.settled()
        self.assertTrue(self.originals[0].stdout.closed)


class OwnedProcessNativeTest(unittest.TestCase):
    def test_t1108_actual_captured_child_returns_exact_bounded_input_after_exit(self):
        record = bytes(range(256))
        result = owned.captured([sys.executable, '-c',
            'import sys; data=sys.stdin.buffer.read(); sys.stdout.buffer.write(data[::-1])'], input=record)
        result.check_returncode()
        self.assertEqual(result.stdout, record[::-1])
