"""T876: read-only native inventory, strict bounds, and no USB mutations."""
from pathlib import Path
import gzip
import json
import os
import random
import struct
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def compile_arguments(binary):
    fixture = ROOT / 'scripts/tests/fixtures/aoa'
    native = ROOT / 'host/accessory'
    return ['-std=c11', '-Wall', '-Wextra', '-Werror',
            '-fsanitize=address,undefined', '-I' + str(fixture), '-I' + str(native),
            str(fixture / 'discovery_fixture.c'), str(native / 'discovery.c'),
            str(native / 'contract.c'), '-o', str(binary)]


def reported_sources(directory):
    sources = set()
    for path in directory.glob('*.gcov.json.gz'):
        with gzip.open(path, 'rt') as stream:
            sources.update(entry['file'] for entry in json.load(stream)['files'])
    return sources


class AccessoryDiscoveryTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='blent-t876-discovery-')
        cls.addClassCleanup(cls.directory.cleanup)
        cls.binary = Path(cls.directory.name) / 'discovery'
        subprocess.run(['cc', *compile_arguments(cls.binary)], check=True, timeout=30)

    def test_t1314_fixture_and_native_counters_remain_distinct(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1314-counters-') as temp:
            directory = Path(temp)
            binary = directory / 'discovery'
            arguments = compile_arguments(binary)
            compiler = os.environ.get('BLENT_REAL_CC', 'cc')
            subprocess.run([compiler, *arguments, '--coverage'], check=True)
            subprocess.run([binary, '0', '64'], check=True, capture_output=True)
            notes = sorted(directory.glob('*.gcno'))
            self.assertEqual(len(notes), 3, 'T1314: source coverage notes collided')
            subprocess.run(['gcov', '--json-format', *map(str, notes)],
                           cwd=directory, check=True, capture_output=True)
            expected = {argument for argument in arguments if argument.endswith('.c')}
            self.assertEqual(reported_sources(directory), expected)

    def run_inventory(self, mode, capacity='64'):
        return subprocess.run([self.binary, str(mode), capacity], capture_output=True, timeout=3)

    def test_t876_fixed_records_filter_roots_hubs_without_opening_devices(self):
        result = self.run_inventory(0)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout[:16], b'BLUI\0\0\0\1\0\3' + b'\0' * 6)
        self.assertEqual(len(result.stdout), 64)
        self.assertEqual(struct.unpack('>BBBBHH7sB', result.stdout[16:32]),
                         (1, 3, 7, 255, 0x1234, 0x5678, bytes(range(1, 8)), 0))
        self.assertIn(b'freed=1 exited=1', result.stderr)
        self.assertEqual(self.run_inventory(1).stdout, b'BLUI\0\0\0\1' + b'\0' * 8)

    def test_t876_native_failure_capacity_and_metadata_bounds(self):
        for mode in range(2, 12):
            result = self.run_inventory(mode)
            self.assertEqual(result.returncode, 2, (mode, result.stderr))
            self.assertEqual(result.stdout, b'')
        result = self.run_inventory(0, '2')
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(result.stdout, b'')
        result = self.run_inventory(12, '512')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(result.stdout), 8208)
        self.assertEqual(result.stdout[8:10], b'\2\0')

    def test_t876_invalid_capacity_rejected_before_usb_initialization(self):
        invalid = ['', '0', '01', '513', '-1', '+1', ' 1', '1x', '1\n', '9' * 512]
        randomizer = random.Random(876)
        invalid.extend(str(randomizer.randrange(513, 2**64)) for _ in range(64))
        for capacity in invalid:
            result = self.run_inventory(0, capacity)
            self.assertEqual(result.returncode, 2, (capacity, result.stderr))
            self.assertIn(b'freed=0 exited=0', result.stderr)
            self.assertEqual(result.stdout, b'')

    def test_t876_output_failure_does_not_report_success(self):
        with open('/dev/full', 'wb') as full:
            result = subprocess.run([self.binary, '0', '64'], stdout=full,
                                    stderr=subprocess.PIPE, timeout=3)
        self.assertEqual(result.returncode, 2, result.stderr)


if __name__ == '__main__':
    unittest.main()
