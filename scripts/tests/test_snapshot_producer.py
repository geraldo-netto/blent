"""T1282: snapshot inventory rejection must precede native display effects."""
from pathlib import Path
import shlex
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class SnapshotProducerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix='blent-t1282-')
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.binary = Path(cls.temporary.name) / 'snapshot'
        flags = subprocess.check_output(['pkg-config', '--cflags', '--libs', 'libdrm'], text=True)
        subprocess.run(['cc', '-std=c11', '-O1', '-g', '-fsanitize=address,undefined',
                        '-fno-sanitize-recover=all', '-fno-pie', '-no-pie',
                        str(ROOT / 'host/tests/snapshot_producer.c'),
                        '-Wl,--wrap=open', '-Wl,--wrap=close', *shlex.split(flags),
                        '-o', str(cls.binary)], check=True)

    def check_case(self, fault, entry, expected):
        result = subprocess.run([str(self.binary), str(fault), entry], capture_output=True, text=True)
        self.assertEqual(result.returncode, expected, result.stderr)
        self.assertNotIn('Sanitizer', result.stderr)

    def test_t1282_empty_missing_and_disconnected_inventory(self):
        for fault in range(1, 10):
            with self.subTest(fault=fault):
                self.check_case(fault, 'discover', 2)
                self.check_case(fault, 'main', 2)

    def test_t1282_revalidate_before_display_effects(self):
        for fault in range(1, 10):
            with self.subTest(fault=fault):
                self.check_case(20 + fault, 'main', 2)

    def test_t1282_valid_inventory_releases_borrowed_objects(self):
        self.check_case(0, 'discover', 0)
        self.check_case(0, 'main', 77)
