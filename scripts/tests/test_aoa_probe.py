"""T846: invalid native probe targets must reject before any USB operation."""
from pathlib import Path
import random
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AoaProbeTargetTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='blent-aoa-contract-')
        cls.addClassCleanup(cls.directory.cleanup)
        root = Path(cls.directory.name)
        source = root / 'target.c'
        source.write_text('''#include "accessory.h"
#include <stdio.h>
int main(int argc,char **argv) {
    if (argc!=3) return 9;
    accessory_target target=accessory_parse_target(argv[1],argv[2]);
    printf("%u %d %u\\n",target.bus,target.count,target.ports[target.count-1]);
    return 0;
}
''')
        cls.binary = root / 'target'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-fsanitize=address,undefined',
                        '-I' + str(ROOT / 'host/accessory'), '-I' + str(ROOT / 'scripts/benchmarks/aoa'), str(source),
                        str(ROOT / 'host/accessory/contract.c'), '-o', str(cls.binary)], check=True)
        cls.echo = cls.build_echo_fixture(root)

    @classmethod
    def build_echo_fixture(cls, root):
        source = root / 'echo.c'
        source.write_text(r'''#define main benchmark_main
#include "main.c"
#undef main
#include <assert.h>
static unsigned char saved_header[24],saved_payload[65536];
static int phase,corrupt;
void accessory_open(accessory_usb *value,accessory_target target) { (void)value;(void)target; }
void accessory_close(accessory_usb *value) { (void)value; }
void accessory_transfer(accessory_usb *value,int receive,unsigned char *data,int length) {
    (void)value;
    assert(receive==(phase>=2));
    switch (phase) {
        case 0: assert(length==24);memcpy(saved_header,data,24);break;
        case 1: assert(length>0 && length<=65536);memcpy(saved_payload,data,(size_t)length);break;
        case 2: assert(length==24);memcpy(data,saved_header,24);if(corrupt==1)data[0]^=1;break;
        case 3: memcpy(data,saved_payload,(size_t)length);if(corrupt==2)data[0]^=1;break;
        default: abort();
    }
    phase=(phase+1)%4;
}
static void golden_vector(void) {
    unsigned char bytes[24],payload[]={1,2,3};
    const unsigned char expected[]={0x42,0x41,0x4f,0x31,1,0x23,0x45,0x67,0x89,0xab,0xcd,0xef,
                                  0,0,0,0,0,0,0,3,0x55,0xbc,0x80,0x1d};
    put32(bytes,0x42414f31);put64(bytes+4,UINT64_C(0x0123456789abcdef));
    put32(bytes+12,0);put32(bytes+16,3);put32(bytes+20,(uint32_t)crc32(0,payload,3));
    assert(!memcmp(bytes,expected,24));
}
int main(int argc,char **argv) {
    assert(argc==2);corrupt=atoi(argv[1]);golden_vector();
    FILE *output=tmpfile();assert(output);
    for(int sequence=0;sequence<6;sequence++) frame(output,1,sequence);
    assert(!fclose(output));return 0;
}
''')
        binary = root / 'echo'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-fsanitize=address,undefined',
                        '-I' + str(ROOT / 'host/accessory'), '-I' + str(ROOT / 'scripts/benchmarks/aoa'), str(source),
                        str(ROOT / 'host/accessory/contract.c'), '-lz', '-o', str(binary)], check=True)
        return binary

    def run_target(self, bus, path):
        return subprocess.run([self.binary, bus, path], capture_output=True, text=True, timeout=2)

    def test_t846_physical_path_boundaries(self):
        for bus, path, expected in [('1', '1', '1 1 1'), ('255', '1.2.3.4.5.6.255', '255 7 255')]:
            result = self.run_target(bus, path)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), expected)

    def test_t846_invalid_paths_reject_without_usb_dependency(self):
        for bus, path in [('0', '1'), ('256', '1'), ('-1', '1'), ('1x', '1'), ('', '1'),
                          ('1', ''), ('1', '0'), ('1', '256'), ('1', '.1'), ('1', '1.'),
                          ('1', '1..2'), ('1', '1/2'), ('1', '1.2.3.4.5.6.7.8'), ('9' * 512, '1')]:
            self.assertEqual(self.run_target(bus, path).returncode, 2, (bus, path))

    def test_t846_bounded_invalid_numeric_fuzz(self):
        randomizer = random.Random(846)
        for _ in range(128):
            invalid = str(randomizer.randrange(256, 2**64))
            self.assertEqual(self.run_target(invalid, '1').returncode, 2)
            self.assertEqual(self.run_target('1', '2.' + invalid).returncode, 2)

    def test_t846_c_wire_golden_vector_payload_boundaries_and_corruption(self):
        for corruption, expected in [('0', 0), ('1', 2), ('2', 2)]:
            result = subprocess.run([self.echo, corruption], capture_output=True, text=True, timeout=2)
            self.assertEqual(result.returncode, expected, result.stderr)


class AoaProbeEnumerationTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='blent-aoa-enumeration-')
        cls.addClassCleanup(cls.directory.cleanup)
        cls.binary = Path(cls.directory.name) / 'enumeration'
        fixture = ROOT / 'scripts/tests/fixtures/aoa'
        adapter = ROOT / 'host/accessory'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        '-fsanitize=address,undefined', '-I' + str(fixture), '-I' + str(adapter),
                        str(fixture / 'enumeration.c'), str(adapter / 'accessory.c'),
                        str(adapter / 'contract.c'), '-o', str(cls.binary)], check=True)

    def test_t846_usb_zero_length_packets_are_bounded_read_events(self):
        fixture = ROOT / 'scripts/tests/fixtures/aoa'
        adapter = ROOT / 'host/accessory'
        binary = Path(self.directory.name) / 'bulk'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        '-fsanitize=address,undefined', '-I' + str(fixture), '-I' + str(adapter),
                        str(fixture / 'bulk.c'), str(adapter / 'accessory.c'),
                        str(adapter / 'contract.c'), '-o', str(binary)], check=True)
        for mode, expected, calls in [(1, 0, 4), (2, 2, 9), (3, 2, 1),
                                      (4, 2, 1), (5, 2, 1), (6, 2, 0), (7, 2, 0), (8, 2, 1)]:
            result = subprocess.run([binary, str(mode)], capture_output=True, text=True, timeout=2)
            self.assertEqual(result.returncode, expected, result.stderr)
            self.assertIn(f'calls={calls}', result.stderr)

    def run_probe(self, error, persistent=0, strict=0, absent=0):
        return subprocess.run([self.binary, str(error), str(persistent), str(strict), str(absent)],
                              capture_output=True, text=True, timeout=2)

    def test_t846_transient_access_and_disappearance_recover(self):
        for error in [-3, -4]:
            for absent in [0, 1]:
                result = self.run_probe(error, absent=absent)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(f'attempts=3 sleeps={2 + absent}', result.stderr)

    def test_t879_replaced_selection_never_sends_control_transfers(self):
        for changed in [1, 2, 3]:
            result = subprocess.run([self.binary, '0', '0', '0', '0', str(changed)],
                                    capture_output=True, text=True, timeout=2)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn('controls=0', result.stderr)

    def test_t854_production_metadata_matches_android_filter(self):
        import xml.etree.ElementTree as ET
        result = subprocess.run([self.binary, '0', '0', '0', '0', '4'],
                                capture_output=True, text=True, timeout=2)
        self.assertEqual(result.returncode, 0, result.stderr)
        profile = ET.parse(ROOT / 'android/app/src/main/res/xml/accessory_filter.xml').getroot()[0].attrib
        for index, field in [(0, 'manufacturer'), (1, 'model'), (3, 'version')]:
            self.assertIn(f'metadata[{index}]={profile[field]}\n', result.stderr)
        self.assertIn('metadata[2]=Blent USB connection\n', result.stderr)
        self.assertIn('approve Blent on tablet.', result.stderr)
        prototype = self.run_probe(0)
        self.assertEqual(prototype.returncode, 0, prototype.stderr)
        self.assertIn('metadata[1]=Blent USB Probe\n', prototype.stderr)

    def test_t879_exact_selection_allows_expected_reenumeration(self):
        result = subprocess.run([self.binary, '0', '0', '0', '0', '4'],
                                capture_output=True, text=True, timeout=2)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('controls=8', result.stderr)

    def test_t846_persistent_open_error_is_bounded(self):
        for error in [-3, -4]:
            result = self.run_probe(error, persistent=1)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn('attempts=101 sleeps=100', result.stderr)
            self.assertIn(f'USB open error {error}', result.stderr)

    def test_t846_initial_open_and_nontransient_errors_remain_strict(self):
        for error in [-3, -4, -1]:
            result = self.run_probe(error, strict=1)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn('attempts=1 sleeps=0', result.stderr)
        result = self.run_probe(-1)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('attempts=2 sleeps=1', result.stderr)


class AccessoryStreamTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='blent-accessory-stream-')
        cls.addClassCleanup(cls.directory.cleanup)
        cls.binary = Path(cls.directory.name) / 'stream'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        '-fsanitize=address,undefined',
                        '-I' + str(ROOT / 'scripts/tests/fixtures/aoa'),
                        '-I' + str(ROOT / 'host/accessory'),
                        str(ROOT / 'scripts/tests/fixtures/aoa/stream_fixture.c'),
                        str(ROOT / 'host/accessory/contract.c'), '-pthread', '-o', str(cls.binary)], check=True)

    def test_t876_inventory_dispatch_does_not_enter_usb_streaming(self):
        result = subprocess.run([self.binary, '24'], capture_output=True, timeout=3)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_t879_stream_entry_preserves_selected_descriptor_arguments(self):
        result = subprocess.run([self.binary, '25'], capture_output=True, timeout=3)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_t846_packet_aligned_usb_writes_terminate_pending_device_read(self):
        for scenario in range(15, 24):
            result = subprocess.run([self.binary, str(scenario)], capture_output=True, timeout=3)
            expected = 0 if scenario in (15, 18, 19) else 2
            self.assertEqual(result.returncode, expected, (scenario, result.stderr.decode()))

    def test_t846_usb_duplex_bounds_timeouts_partial_io_and_process_retirement(self):
        for scenario in range(15):
            expected = 2 if scenario in (3, 4, 5, 6, 8, 9, 10, 12, 14) else 0
            result = subprocess.run([self.binary, str(scenario)], capture_output=True, timeout=3)
            self.assertEqual(result.returncode, expected, (scenario, result.stderr.decode()))


if __name__ == '__main__':
    unittest.main()
