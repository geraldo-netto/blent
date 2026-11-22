"""T1710: all installation routes derive their unit from one policy owner."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def package_fixture(root):
    source = root / 'blent-1.2.3'
    files = ['scripts/setup-system.sh', 'scripts/gpu-boot-order.sh', 'scripts/setup-evdi.sh',
             'scripts/blent.desktop', 'packaging/icons/blent.svg',
             'packaging/icons/blent-pen.svg', 'packaging/blent-evdi.conf',
             'packaging/blent-modules.conf', 'packaging/60-blent-uinput.rules',
             'LICENSE', 'licenses/libevdi-LGPL-2.1.txt']
    for name in files:
        target = source / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, target)
    for name in ['blent', 'blent-gui', 'blent-audio', 'blent-usb-broker', 'blent-usb-io']:
        target = source / 'target/release' / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text('fixture')
    (source / 'host/evdi').mkdir(parents=True)
    (source / 'host/evdi/evdi_helper').write_text('fixture')
    for name in ['blent.service', 'write-systemd-service.sh']:
        shutil.copyfile(ROOT / 'scripts' / name, source / 'scripts' / name)
    (source / 'scripts/blent.service').write_text(
        (source / 'scripts/blent.service').read_text().replace('Description=', 'Description=T1710 canonical '))
    docs = source / 'scripts/copy-distribution-docs.sh'
    docs.write_text('#!/bin/sh\nexit 0\n')
    docs.chmod(0o755)
    return source


class ServiceTemplateTest(unittest.TestCase):
    def test_t1710_arch_package_uses_canonical_policy(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1710-') as directory:
            root = Path(directory)
            source = package_fixture(root)
            output = root / "package 'literal % $ output"
            command = '''source "$1"
mkdir -p "$srcdir/evdi-$_evdi/library"
printf fixture > "$srcdir/evdi-$_evdi/library/libevdi.so.1.15.0"
package
'''
            result = subprocess.run(['bash', '-eu', '-c', command, 'fixture',
                                     str(ROOT / 'packaging/arch/PKGBUILD')], cwd=root,
                                    env=dict(os.environ, pkgdir=str(output), srcdir=str(root)),
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            expected = subprocess.check_output(['bash', str(source / 'scripts/write-systemd-service.sh'),
                '/usr/bin/blent', str(source / 'scripts/blent.service')], text=True)
            unit = output / 'usr/lib/systemd/user/blent.service'
            self.assertEqual(unit.read_text(), expected, 'T1710: package policy diverges from canonical renderer')
            self.assertEqual(unit.stat().st_mode & 0o777, 0o644)
            self.assertIn('SendSIGKILL=no', expected)
            self.assert_setup_artifacts(source, output)

    def assert_setup_artifacts(self, source, output):
        # T1625: package resources remain byte-identical to the canonical owner.
        shared = output / 'usr/share/blent'
        for name in ['setup-system.sh', 'setup-evdi.sh', 'gpu-boot-order.sh']:
            self.assertEqual((shared/name).read_bytes(), (source/'scripts'/name).read_bytes())
        for name in ['blent-evdi.conf', 'blent-modules.conf', '60-blent-uinput.rules']:
            self.assertEqual((shared/'setup'/name).read_bytes(), (source/'packaging'/name).read_bytes())

    def test_t1710_only_one_authoritative_unit_template(self):
        self.assertFalse((ROOT / 'packaging/blent.service').exists())
        self.assertNotIn('packaging/blent.service', (ROOT / 'scripts/stage-linux-bundle.sh').read_text())


if __name__ == '__main__':
    unittest.main()
