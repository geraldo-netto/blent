"""T1113: compile the real benchmark and exercise its original frame borrows."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'benchmarks'))
import profile_usb_pipeline as pipeline
import profile_process as owned

IDLE = pipeline.module('idle-writer')
ROOT = Path(__file__).resolve().parents[2]


def build(folder, interval):
    try:
        IDLE.build(folder, interval)
    except subprocess.CalledProcessError as failure:
        raise AssertionError((folder / 'build.log').read_text()) from failure


class IdleWriterNativeTest(unittest.TestCase):
    def test_t1113_current_and_trial_writer_compile_with_current_frame_contract(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1113-compile-') as temporary:
            for interval in (200, 500):
                with self.subTest(interval=interval):
                    build(Path(temporary) / str(interval), interval)

    def test_t1113_bounded_publication_withdrawal_and_retained_original_lease(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1113-native-') as temporary:
            folder = Path(temporary) / 'candidate'
            build(folder, 200)
            binary = folder / 'ownership'
            command = ['cc', '-std=c11', '-O1', '-g', '-pthread', '-Wall', '-Wextra', '-Werror',
                       '-fsanitize=address,undefined', '-fno-pie', '-no-pie',
                       '-I', str(folder), '-I', str(ROOT / 'host/evdi'),
                       str(ROOT / 'scripts/tests/idle_writer_native.c'),
                       str(ROOT / 'host/evdi/frame_exchange.c'),
                       str(ROOT / 'host/evdi/fifo_writer.c'),
                       str(ROOT / 'host/evdi/idle_control.c'), '-o', str(binary)]
            logfile = folder / 'native.log'
            self.assertEqual(owned.logged(command, dict(os.environ), logfile, ROOT), 0,
                             logfile.read_text())
            self.assertEqual(owned.logged([str(binary)], dict(os.environ), logfile, ROOT), 0,
                             logfile.read_text())
