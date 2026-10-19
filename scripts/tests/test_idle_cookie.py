"""T1117: independent native acceptance of padded source-cookie fields."""
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'benchmarks'))
import profile_process as owned

ROOT = Path(__file__).resolve().parents[2]


class IdleCookieTest(unittest.TestCase):
    def test_t1117_production_c_accepts_numeric_cookie_padding(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1117-cookie-') as temporary:
            folder = Path(temporary)
            binary = folder / 'cookie'
            logfile = folder / 'native.log'
            command = ['cc', '-std=c11', '-O1', '-g', '-pthread', '-Wall', '-Wextra', '-Werror',
                       '-fsanitize=address,undefined', '-fno-pie', '-no-pie',
                       '-I', str(ROOT / 'host/evdi'), str(ROOT / 'scripts/tests/idle_cookie.c'),
                       str(ROOT / 'host/evdi/frame_exchange.c'), str(ROOT / 'host/evdi/fifo_writer.c'),
                       '-o', str(binary)]
            self.assertEqual(owned.logged(command, dict(os.environ), logfile, ROOT), 0,
                             logfile.read_text())
            self.assertEqual(owned.logged([str(binary)], dict(os.environ), logfile, ROOT), 0,
                             logfile.read_text())
