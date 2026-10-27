"""Linux CI must deliver its native executables and their coverage evidence."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/build.yml'


def step(name):
    steps = re.split(r'(?m)^      - ', WORKFLOW.read_text())
    return next(value for value in steps if value.startswith('name: ' + name + '\n'))


class HostArtifactTest(unittest.TestCase):
    def test_t1312_upload_contains_every_native_executable(self):
        upload = step('Upload host binaries')
        paths = re.findall(r'(?m)^            (\S+)$', upload)
        self.assertEqual(set(paths), {
            'target/release/blent', 'target/release/blent-gui',
            'target/release/blent-audio', 'target/release/blent-usb-broker',
            'target/release/blent-usb-io', 'host/evdi/evdi_helper',
        })

    def test_t1312_usb_helper_build_precedes_upload(self):
        workflow = WORKFLOW.read_text()
        build = step('Build USB helper (T1312)')
        self.assertIn('run: make build-usb-helper', build)
        self.assertLess(workflow.index(build), workflow.index(step('Upload host binaries')))

    def test_t1312_install_usb_build_headers(self):
        dependencies = step('Install system dependencies')
        self.assertRegex(dependencies, r'\blibusb-1\.0-0-dev\b')


class HostCoverageTest(unittest.TestCase):
    def test_t1313_collect_usb_helper_counters(self):
        collect = step('Enforce USB helper C function coverage (T1313)')
        self.assertIn("scripts/coverage/capture_check.py", collect)
        self.assertIn("'${{ runner.temp }}/blent-usb-coverage'", collect)
        self.assertIn('--scope host/accessory/', collect)
        self.assertNotIn('continue-on-error', collect)

    def test_t1313_preserve_usb_helper_counters(self):
        upload = step('Preserve host coverage evidence')
        self.assertIn('if: always()', upload)
        self.assertIn('${{ runner.temp }}/blent-usb-coverage', upload)

    def test_t1313_merge_usb_helper_counters(self):
        merge = step('Enforce the combined native gate (T497)')
        self.assertIn('--gcov evidence/host/blent-usb-coverage/native', merge)
        self.assertNotIn('--report-only', merge)
