"""T1711: one setup policy, isolated files and native command boundaries."""
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class SystemSetupTest(unittest.TestCase):
    def test_t1711_every_route_uses_the_shared_setup(self):
        for path in ['Makefile', 'scripts/install.sh', 'packaging/arch/blent.install']:
            with self.subTest(route=path):
                self.assertIn('setup-system.sh', (ROOT / path).read_text())

    def exercise(self, root, fail='', evdi=0, gpu=0):
        program = ROOT / 'scripts/setup-system.sh'
        scripts = root / 'scripts'
        scripts.mkdir(exist_ok=True)
        for name, status in [('gpu-boot-order.sh', gpu), ('setup-evdi.sh', evdi)]:
            (scripts / name).write_text('#!/bin/sh\nprintf "%s\\n" ' + name + ' >> "$SETUP_LOG"\nexit ' + str(status) + '\n')
        source = '''source "$1"
modprobe() { printf 'modprobe %s\\n' "$*" >> "$SETUP_LOG"; [[ $SETUP_FAIL != module ]]; }
udevadm() { printf 'udevadm %s\\n' "$*" >> "$SETUP_LOG"; [[ $SETUP_FAIL != udev ]]; }
setup_system "$2/etc" "$3/packaging" "$2/scripts" "$2/drm" "$2/init"
'''
        result = subprocess.run(['bash', '-eu', '-c', source, 'fixture', str(program), str(root), str(ROOT)],
            env=dict(os.environ, SETUP_LOG=str(root/'calls'), SETUP_FAIL=fail),capture_output=True,text=True)
        return result

    def test_t1711_ready_and_deferred_setup_have_consistent_files(self):
        for evdi, gpu in [(0, 0), (1, 0), (0, 1), (1, 1)]:
            with self.subTest(evdi=evdi,gpu=gpu), tempfile.TemporaryDirectory() as directory:
                root=Path(directory)
                result=self.exercise(root,evdi=evdi,gpu=gpu)
                self.assertEqual(result.returncode,0,result.stdout+result.stderr)
                self.assertEqual((root/'etc/modules-load.d/blent.conf').read_bytes(),(ROOT/'packaging/blent-modules.conf').read_bytes())
                self.assertEqual((root/'etc/udev/rules.d/60-blent-uinput.rules').read_bytes(),(ROOT/'packaging/60-blent-uinput.rules').read_bytes())
                calls=(root/'calls').read_text()
                self.assertLess(calls.index('gpu-boot-order.sh'),calls.index('setup-evdi.sh'))
                self.assertIn('udevadm control --reload',calls)
                self.assertIn('udevadm trigger --name-match=uinput',calls)
                self.assertEqual('EVDI activation deferred' in result.stderr,evdi != 0)

    def test_t1711_reinstall_preserves_evdi_but_updates_owned_rule(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);config=root/'etc/modprobe.d/blent-evdi.conf'
            config.parent.mkdir(parents=True);config.write_text('options evdi initial_device_count=4 loglevel=3\n')
            rule=root/'etc/udev/rules.d/60-blent-uinput.rules';rule.parent.mkdir(parents=True);rule.write_text('obsolete owned rule')
            result=self.exercise(root)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual(config.read_text(),'options evdi initial_device_count=4 loglevel=3\n')
            self.assertEqual(rule.read_bytes(),(ROOT/'packaging/60-blent-uinput.rules').read_bytes())

    def test_t1711_native_activation_failures_are_reported(self):
        for fail in ['module','udev']:
            with self.subTest(fail=fail),tempfile.TemporaryDirectory() as directory:
                result=self.exercise(Path(directory),fail=fail)
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertIn('manual activation',result.stderr)

    def test_t1711_configuration_failure_stops_activation(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'etc').write_text('not a directory')
            result=self.exercise(root)
            self.assertNotEqual(result.returncode,0)
            self.assertFalse((root/'calls').exists())

    def test_t1711_entry_validates_arguments_and_passes_literal_paths(self):
        program = ROOT / 'scripts/setup-system.sh'
        for args in [[], ['one', 'two']]:
            result = subprocess.run(['bash', str(program), *args], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn('Usage:', result.stderr)
        command = '''source "$1"
setup_system() { printf '%s\n' "$@"; }
system_setup_main "$2"
'''
        resource = "resource ' % $ path"
        result = subprocess.run(['bash', '-eu', '-c', command, 'fixture', str(program), resource],capture_output=True,text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), ['/etc', resource, str(ROOT/'scripts'), '/sys/class/drm', '/run/systemd/system'])

    def test_t1711_boot_publication_failure_and_concurrent_owner_preserve_files(self):
        from system_setup_fixture import SOURCE
        for boundary in ['install', 'ln']:
            with self.subTest(boundary=boundary), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                failure = 'install() { return 7; }' if boundary == 'install' else 'ln() { printf "external\n" > "${@: -1}"; return 1; }'
                command = SOURCE + failure + '\npreserve_evdi_config "$1" "$2"\n'
                result = subprocess.run(['bash', '-eu', '-c', command, 'fixture', str(ROOT/'packaging/blent-evdi.conf'), str(root/'config')],capture_output=True,text=True)
                self.assertEqual(result.returncode, 7 if boundary == 'install' else 0, result.stderr)
                self.assertEqual(list(root.glob('.blent-evdi.*')), [])
                if boundary == 'ln': self.assertEqual((root/'config').read_text(), 'external\n')
                else: self.assertFalse((root/'config').exists())


if __name__=='__main__':unittest.main()
