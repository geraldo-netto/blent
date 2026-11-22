"""T1288/T1720: current-policy builds and independent pixel/checksum replay."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'benchmarks'))
import profile_usb_pipeline as pipeline

SHARED = pipeline.module('shared-damage')
ROOT = Path(__file__).resolve().parents[2]


class SharedDamageNativeTest(unittest.TestCase):
    def test_t1720_historical_baseline_option_is_rejected(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1720-cli-') as temporary:
            output = Path(temporary) / 'output'
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/benchmarks/shared-damage.py'),
                                     str(output), '--baseline', 'not-a-revision'],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn('unrecognized arguments: --baseline', result.stderr)
            self.assertFalse(output.exists())

    def test_t1288_current_policy_variants_build(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1288-build-') as temporary:
            SHARED.build(Path(temporary))

    def test_t1288_bounded_replay_matches_independent_pixels_and_checksum(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1288-replay-') as temporary:
            binary = Path(temporary) / 'replay'
            sources = ROOT / 'host/evdi'
            command = ['cc', '-std=c11', '-D_GNU_SOURCE=', '-O1', '-g', '-pthread',
                       '-ffunction-sections', '-fdata-sections', '-Wl,--gc-sections',
                       '-fsanitize=address,undefined', '-fno-sanitize-recover=all',
                       '-fno-pie', '-no-pie', '-I', str(sources),
                       '-include', str(sources / 'frame_exchange_private.h'),
                       '-include', str(sources / 'conversion_private.h'),
                       str(ROOT / 'scripts/tests/shared_damage_native.c'),
                       *[str(sources / name) for name in
                         ['conversion.c', 'frame_exchange.c', 'raw_ring.c', 'cursor_overlay.c', 'capture_cadence.c']],
                       '-o', str(binary)]
            for full in [0, 1]:
                subprocess.run([*command, f'-DBLENT_BENCH_FULL_DAMAGE={full}'], check=True)
                subprocess.run([str(binary)], check=True)
