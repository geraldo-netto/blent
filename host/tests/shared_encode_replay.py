#!/usr/bin/env python3
"""T793 permanent native integration: fresh generator, FIFO/shared encode + decode.

Invoked by shared_encode_replay.rs in the normal inproc-encoder test suite.
Requires cached Cargo dependencies, stock libavcodec headers and ffmpeg/libx264.
Missing native tools fail this test; no downloads or success-by-skip.
"""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('replay', ROOT / 'scripts/benchmarks/shared-encode.py')
REPLAY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REPLAY)


class GeneratedReplayTests(unittest.TestCase):
    def test_t793_offline_generated_build_and_fifo_shared_equivalence(self):
        with tempfile.TemporaryDirectory(prefix='blent-t793-') as temporary:
            folder = Path(temporary)
            REPLAY.build(folder)
            target = Path(os.environ.get('CARGO_TARGET_DIR', folder / 'target'))
            binary = target / 'release/blent-shared-encode-bench'
            for width, height in [(64, 48), (66, 50)]:
                with self.subTest(width=width, height=height):
                    encoded = []
                    decoded = []
                    for mode in ['fifo', 'shared']:
                        stream = folder / f'{width}-{mode}.h264'
                        result = subprocess.run(
                            [str(binary), str(folder / 'producer'), mode, str(width),
                             str(height), '3', '1', str(stream)], check=False,
                            capture_output=True, text=True, timeout=30)
                        self.assertEqual(result.returncode, 0, result.stderr)
                        report = json.loads(result.stdout)
                        self.assertEqual(report['mode'], mode)
                        self.assertEqual(len(report['frames']), 3)
                        self.assertEqual([row['sequence'] for row in report['frames']], [60, 61, 62])
                        encoded.append(stream.read_bytes())
                        decoded.append(subprocess.check_output(
                            ['ffmpeg', '-v', 'error', '-i', str(stream), '-f', 'rawvideo',
                             '-pix_fmt', 'yuv420p', 'pipe:1'], timeout=30))
                    self.assertTrue(encoded[0])
                    self.assertEqual(encoded[0], encoded[1])
                    self.assertEqual(decoded[0], decoded[1])
                    frame_size = width * height * 3 // 2
                    self.assertEqual(len(decoded[0]), 123 * frame_size)
                    self.assertNotEqual(decoded[0][:frame_size], decoded[0][-frame_size:])


if __name__ == '__main__':
    unittest.main()
