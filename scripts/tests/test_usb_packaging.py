"""T861: production USB executables survive each Linux distribution boundary."""
from pathlib import Path
import os
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import test_appimage
from test_appimage import build, elf, ffmpeg_bundle, REPO
from test_installer import SOURCE
from shell_fixture import run as run_shell

USB = ('blent-usb-broker', 'blent-usb-io')


class UsbPackagingTest(unittest.TestCase):
    def bundle(self, root):
        binary = root / 'input'
        binary.mkdir()
        for name in ('blent', 'blent-gui', 'blent-audio', 'evdi_helper', 'libevdi.so.1.15.0', *USB):
            (binary / name).write_text(name)
        return binary

    def test_t861_tar_layout_requires_and_preserves_both_usb_programs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            binary = self.bundle(root)
            destination = root / 'bundle'
            result = subprocess.run(['bash', 'scripts/stage-linux-bundle.sh', str(binary),
                str(binary/'evdi_helper'), str(binary/'libevdi.so.1.15.0'), str(destination)],
                cwd=REPO, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            for name in USB:
                self.assertEqual((destination/'bin'/name).read_text(), name)
            self.assertTrue((destination/'licenses/accessory-transport/ring-0.17.14/LICENSE').is_file())

    def test_t861_partial_tar_inputs_are_rejected(self):
        for missing in USB:
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                binary = self.bundle(root)
                (binary/missing).unlink()
                result = subprocess.run(['bash', 'scripts/stage-linux-bundle.sh', str(binary),
                    str(binary/'evdi_helper'), str(binary/'libevdi.so.1.15.0'), str(root/'out')],
                    cwd=REPO, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0, 'T861 partial USB package accepted')

    def test_t861_user_install_copies_sibling_usb_programs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            binary = self.bundle(root)
            staged = root / 'staged'
            staged.mkdir()
            script = SOURCE + '\nPROJECT_DIR=$1\nstage_install_binaries "$2" "$3"\n'
            result = run_shell(script, [str(root), str(binary), str(staged)], cwd=REPO,
                               capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            for name in USB:
                self.assertEqual((staged/name).read_text(), name)
                self.assertTrue(os.access(staged/name, os.X_OK))

    def test_t861_partial_user_install_is_rejected(self):
        for missing in USB:
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                binary = self.bundle(root)
                (binary/missing).unlink()
                staged = root/'staged'; staged.mkdir()
                result = run_shell(SOURCE + '\nPROJECT_DIR=$1\nstage_install_binaries "$2" "$3"\n',
                    [root, binary, staged], cwd=REPO, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0, 'T861 partial native install accepted')

    def test_t861_appimage_validates_usb_abi_loader_and_transitive_libusb(self):
        fixture = test_appimage.AppImageTest(); fixture.setUp()
        try:
            bundle, _ = fixture.t497_bundle()
            for name in USB:
                fixture.write(bundle/'bin'/name, name)
            dependency = fixture.write(fixture.root/'libusb-1.0.so.0', 'stock libusb')
            destination = fixture.root/'usb.AppDir'
            with patch.object(build.shutil, 'which', return_value=str(fixture.root/'stock/adb')), \
                    patch.object(ffmpeg_bundle, 'verify'), patch.object(elf, 'verify_abi') as abi, \
                    patch.object(elf, 'closure', return_value={'libusb-1.0.so.0': dependency}) as closure, \
                    patch.object(elf, 'set_app_rpath') as rpath, patch.object(elf, 'check_loaded') as loaded:
                paths = build.stage(REPO, bundle, destination, fixture.root/'pinned')
            self.assertIn(dependency, paths)
            for name in USB:
                target = destination/'usr/bin'/name
                self.assertEqual(target.read_text(), name)
                abi.assert_any_call(target)
                self.assertIn(target, closure.call_args.args[0])
                rpath.assert_any_call(target, helper=False)
                loaded.assert_any_call(target)
        finally:
            fixture.doCleanups()

class UsbDependencyTest(unittest.TestCase):
    def test_t861_installer_requests_native_usb_runtime(self):
        import test_installer
        fixture = test_installer.InstallerTest()
        cases = [('debian', 'libusb-1.0-0'), ('arch', 'libusb')]
        stubs = '\nsudo() { printf "%s\\n" "$*"; }\npacman() { return 0; }\nhas_evdi_library() { return 0; }\n'
        for distro, package in cases:
            with self.subTest(distro=distro):
                output = fixture.run_installer(stubs + f'install_distro_deps "{distro}"\n')
                self.assertIn(package, output)

    def test_t861_arch_recipe_installs_both_usb_siblings(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            project = root/'blent-1.2.3'; project.mkdir()
            for name in ('blent', 'blent-gui', 'blent-audio', *USB):
                path = project/'target/release'/name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text(name)
            script = (REPO/'packaging/arch/PKGBUILD').read_text()
            # Exercise real recipe with only its unrelated display/docs effects stubbed.
            suffix = r'''
srcdir=$1
pkgdir=$2
install() {
  case "$2" in target/release/*) command install "$@" ;; *) : ;; esac
}
ln() { :; }
package
'''
            docs = project/'scripts/copy-distribution-docs.sh'; docs.parent.mkdir(); docs.write_text('#!/bin/sh\nexit 0\n'); docs.chmod(0o755)
            result = run_shell(script + suffix, [root, root/'package'], cwd=root, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            for name in USB:
                self.assertEqual((root/'package/usr/bin'/name).read_text(), name)

    def test_t861_unavailable_package_transactions_retain_actionable_guidance(self):
        import test_installer
        fixture = test_installer.InstallerTest()
        stubs = '\nsudo() { [[ $* == \"apt-get update\" ]]; }\nhas_evdi_library() { return 1; }\n'
        for distro, message in [('debian', 'Check the package names'), ('opensuse', 'Unknown distro. Install manually:')]:
            with self.subTest(distro=distro):
                output = fixture.run_installer(stubs + f'install_distro_deps "{distro}"\n')
                self.assertIn(message, output)

    def test_t861_arch_dependency_failure_retains_each_explicit_aur_route(self):
        import test_installer
        fixture = test_installer.InstallerTest()
        stubs = r'''
sudo() { :; }
pacman() { return 1; }
yay() { printf 'selected yay\n'; }
paru() { printf 'selected paru\n'; }
command() {
    if [[ $1 == -v && ( $2 == yay || $2 == paru ) ]]; then
        [[ $2 == $TEST_AUR ]]
    else builtin command "$@"; fi
}
'''
        for route, message in [('yay', 'selected yay'), ('paru', 'selected paru'), ('absent', 'evdi lives in the AUR')]:
            with self.subTest(route=route):
                output = fixture.run_installer(stubs + f'TEST_AUR={route}\ninstall_distro_deps arch\n')
                self.assertIn(message, output)
