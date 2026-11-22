"""T1609: real native locks use the explicitly selected private runtime."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AccessoryRuntimeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='blent-t1609-')
        cls.addClassCleanup(cls.directory.cleanup)
        cls.root = Path(cls.directory.name)
        cls.binary = cls.root/'lock'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        '-fsanitize=address,undefined', '-I'+str(ROOT/'host/accessory'),
                        str(ROOT/'scripts/tests/fixtures/aoa/runtime_lock.c'),
                        str(ROOT/'host/accessory/contract.c'), '-o', str(cls.binary)], check=True)

    def command(self, port='255', bus='254'):
        # The mapped UID has no /run/user entry; never use an installed probe lock.
        return ['/usr/bin/unshare', '--user', '--map-root-user', str(self.binary), bus, port]

    def test_t1609_relocated_runtime_lock_excludes_replacement_until_original_exit(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            runtime = Path(directory)/('long-private-runtime-'*8)
            runtime.mkdir(mode=0o700)
            env = dict(os.environ, BLENT_USB_RUNTIME_DIR=str(runtime))
            with subprocess.Popen(self.command(), env=env, stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE) as original:
                try:
                    self.assertEqual(original.stdout.readline(), b'locked\n')
                    self.assertTrue((runtime/'blent-aoa-probe-254-255').is_file())
                    denied = subprocess.run(self.command(), env=env, input=b'\n', capture_output=True)
                    self.assertEqual(denied.returncode, 2)
                    self.assertIn(b'another probe still owns', denied.stderr)
                    other = subprocess.run(self.command('254'), env=env, input=b'\n', capture_output=True)
                    self.assertEqual(other.returncode, 0, other.stderr)
                finally:
                    original.communicate(b'\n')
            replacement = subprocess.run(self.command(), env=env, input=b'\n', capture_output=True)
            self.assertEqual(replacement.returncode, 0, replacement.stderr)

    def test_t1609_missing_and_relative_runtime_are_rejected(self):
        for path in ['', 'relative', str(self.root/'missing')]:
            result = subprocess.run(self.command(), input=b'\n', capture_output=True,
                                    env=dict(os.environ, BLENT_USB_RUNTIME_DIR=path))
            self.assertEqual(result.returncode, 2)
            self.assertIn(b'runtime', result.stderr)

    def test_t1609_full_target_extent_and_runtime_validation(self):
        with tempfile.TemporaryDirectory(dir=self.root) as directory:
            runtime = Path(directory)
            env = dict(os.environ, BLENT_USB_RUNTIME_DIR=directory)
            for bus, ports in [('1', '1'), ('255', '255.255.255.255.255.255.255')]:
                result = subprocess.run(self.command(ports, bus), env=env, input=b'\n', capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue((runtime/f'blent-aoa-probe-{bus}-{ports.replace(".", "-")}').is_file())
            link = runtime/'link'
            link.symlink_to(runtime, target_is_directory=True)
            ordinary = runtime/'file'
            ordinary.write_text('not a directory')
            shared = runtime/'shared'
            shared.mkdir(mode=0o755)
            for path in [link, ordinary, shared]:
                result = subprocess.run(self.command(), input=b'\n', capture_output=True,
                                        env=dict(env, BLENT_USB_RUNTIME_DIR=str(path)))
                self.assertEqual(result.returncode, 2, result.stderr)
            env.pop('BLENT_USB_RUNTIME_DIR')
            result = subprocess.run(self.command(), input=b'\n', capture_output=True, env=env)
            self.assertEqual(result.returncode, 2)
            self.assertIn(b'runtime', result.stderr)


if __name__ == '__main__':
    unittest.main()
