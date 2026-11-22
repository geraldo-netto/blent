"""T1808: GUI and release packaging use a compatible, ABI-stable toolchain."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


class PackagingToolchainTest(unittest.TestCase):
    def test_t1808_pinned_toolchain_meets_gui_minimum_on_debian_12(self):
        source = (ROOT/'packaging/ci/Dockerfile').read_text()
        image = re.search(r'^FROM rust:(\d+)\.(\d+)-bookworm@sha256:([0-9a-f]{64})$', source, re.M)
        self.assertIsNotNone(image, 'T1808: pin the Debian 12 image by digest')
        self.assertGreaterEqual(tuple(map(int, image.group(1, 2))), (1, 95))
        self.assertIn('Rust ' + '.'.join(image.group(1, 2)) + ' on Debian 12',
                      (ROOT/'docs/development.md').read_text())
