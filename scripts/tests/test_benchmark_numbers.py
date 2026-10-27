"""T1286: bounded parsing must complete before native workload admission."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'benchmarks'))
import profile_usb_pipeline as pipeline

ROOT = Path(__file__).resolve().parents[2]
SHARED = pipeline.module('shared-damage')


def compile_harness(folder, name):
    sources = ROOT / 'host/evdi'
    (folder / 'candidate_writer.c').write_bytes((sources / 'writer.c').read_bytes())
    extra = ['-DEXCHANGE_SOURCE="frame_exchange.c"']
    if name == 'idle_writer':
        extra = [str(sources / p) for p in ['frame_exchange.c', 'fifo_writer.c', 'idle_control.c']]
    if name == 'shared-damage':
        extra = [*SHARED.fixture_headers(sources), *[str(sources / p) for p in
                 ['conversion.c', 'frame_exchange.c', 'raw_ring.c', 'cursor_overlay.c']]]
    binary = folder / name
    subprocess.run(['cc', '-std=c11', '-O1', '-g', '-pthread',
                    '-ffunction-sections', '-fdata-sections', '-Wl,--gc-sections',
                    '-fsanitize=address,undefined', '-fno-sanitize-recover=all', '-fno-pie', '-no-pie',
                    '-Wl,--wrap=pthread_condattr_init', '-Wl,--wrap=socketpair',
                    '-I', str(sources), '-I', str(folder),
                    str(ROOT / f'scripts/benchmarks/{name}.c'),
                    str(ROOT / 'scripts/tests/benchmark_admission.c'), *extra,
                    '-o', str(binary)], check=True)
    return binary


class BenchmarkNumbersTest(unittest.TestCase):
    def check_arguments(self, binary, arguments, expected):
        result = subprocess.run([str(binary), *arguments], capture_output=True, text=True)
        self.assertEqual(result.returncode, expected, (arguments, result.stderr))
        self.assertEqual(result.stdout, '')
        self.assertNotIn('Sanitizer', result.stderr)

    def invalid_field(self, binary, valid, index, boundaries):
        invalid = ['', 'junk', '1junk', '0x10', '1.5', '2147483648', '-2147483649',
                   '9999999999999999999999999999999', *boundaries]
        for value in invalid:
            arguments = list(valid)
            arguments[index] = value
            with self.subTest(binary=binary.name, arguments=arguments):
                self.check_arguments(binary, arguments, 2)

    def test_t1286_malformed_overflow_and_range_rejected_before_admission(self):
        cases = [('frame-retirement', ['1'], [['0', '-1', '2000']]),
                 ('shared-damage', ['1', '0'], [['0', '-1', '5'], ['-1', '3']]),
                 ('idle_writer', ['30', '0'], [['29', '31', '59', '61'], ['-1', '2']])]
        with tempfile.TemporaryDirectory(prefix='blent-t1286-invalid-') as temporary:
            for name, valid, boundaries in cases:
                binary = compile_harness(Path(temporary), name)
                for index, limits in enumerate(boundaries):
                    self.invalid_field(binary, valid, index, limits)

    def test_t1286_valid_boundaries_reach_admission(self):
        cases = [('frame-retirement', [('1',), ('1999',)]),
                 ('shared-damage', [(str(scale), str(mode)) for scale in range(1, 5) for mode in range(3)]),
                 ('idle_writer', [('30', '0'), ('30', '1'), ('60', '0'), ('60', '1')])]
        with tempfile.TemporaryDirectory(prefix='blent-t1286-valid-') as temporary:
            for name, arguments in cases:
                binary = compile_harness(Path(temporary), name)
                for values in arguments:
                    with self.subTest(binary=name, arguments=values):
                        self.check_arguments(binary, values, 77)
