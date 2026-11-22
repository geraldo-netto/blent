"""T1700: compile real capture code with production optimization and warnings."""
from pathlib import Path
import subprocess
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]


class CaptureReleaseCompileTest(unittest.TestCase):
    def test_t1700_capture_has_proven_latency_index_bounds_at_o3(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1700-') as directory:
            result = subprocess.run([
                'gcc', '-std=c11', '-O3', '-Wall', '-Wextra', '-Werror',
                '-Ihost/evdi', '-c', 'host/evdi/capture.c',
                '-o', str(Path(directory) / 'capture.o'),
            ], cwd=REPO, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
