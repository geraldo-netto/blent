"""T1412: shipped unit policies retain owners through delayed native cleanup."""
import configparser
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
TEMPLATES = ('scripts/blent.service',)


def service_policy(template):
    parser = configparser.ConfigParser(interpolation=None)
    parser.optionxform = str
    parser.read(ROOT / template)
    return parser['Service']


def run(*args):
    return subprocess.run(args, capture_output=True, text=True, check=True)


def await_file(path, seconds=10):
    deadline = time.monotonic() + seconds
    while not path.exists():
        if time.monotonic() >= deadline:
            raise AssertionError(f'Service did not publish {path.name}')
        time.sleep(0.01)


class ServiceRetirementTest(unittest.TestCase):
    def test_t1412_templates_and_renderer_preserve_cleanup_policy(self):
        for template in TEMPLATES:
            with self.subTest(template=template):
                policy = service_policy(template)
                self.assertEqual(policy['TimeoutStopSec'], 'infinity')
                self.assertEqual(policy.get('KillMode'), 'mixed')
                self.assertEqual(policy.get('SendSIGKILL'), 'no')
                self.assertNotIn('ExecStop', policy)
                rendered = run('bash', str(ROOT / 'scripts/write-systemd-service.sh'),
                               '/tmp/T1412 daemon', str(ROOT / template)).stdout
                self.assertIn('TimeoutStopSec=infinity\n', rendered)
                self.assertIn('KillMode=mixed\n', rendered)
                self.assertNotIn('ExecStop=', rendered)

    def test_t1412_native_stop_retains_original_owner_past_former_timeout(self):
        available = subprocess.run(['systemctl', '--user', 'show-environment'],
                                   capture_output=True)
        if available.returncode:
            self.skipTest('Native user systemd manager unavailable; policy test still runs')
        for template in TEMPLATES:
            with self.subTest(template=template):
                self.exercise_native(template)

    def test_t1412_native_restart_waits_for_original_settlement(self):
        available = subprocess.run(['systemctl', '--user', 'show-environment'],
                                   capture_output=True)
        if available.returncode:
            self.skipTest('Native user systemd manager unavailable')
        self.exercise_native(TEMPLATES[0], 'restart')

    def exercise_native(self, template, action='stop'):
        unit = 'blent-t1412-' + uuid.uuid4().hex
        with tempfile.TemporaryDirectory(prefix='blent-t1412-') as directory:
            root = Path(directory)
            try:
                self.start_owned_unit(unit, root, service_policy(template))
                await_file(root / 'ready')
                run('systemctl', '--user', '--no-block', action, unit)
                await_file(root / 'stopping')
                # Observation bound, not a cleanup deadline. Original policy kills at 15s.
                time.sleep(16)
                self.assert_retained(unit, root)
                self.assertFalse((root / 'replacement').exists())
                (root / 'release').touch()
                if action == 'restart':
                    await_file(root / 'replacement')
            finally:
                (root / 'release').touch()
                self.stop_owned_unit(unit, root)
                subprocess.run(['systemctl', '--user', 'reset-failed', unit],
                               capture_output=True)
            self.assertTrue((root / 'settled').exists())

    def stop_owned_unit(self, unit, root):
        stopped = subprocess.run(['systemctl', '--user', 'stop', unit],
                                 capture_output=True, text=True)
        if stopped.returncode:
            # A successful one-shot replacement can already be garbage-collected.
            self.assertEqual(stopped.returncode, 5, stopped.stderr)
            self.assertTrue((root / 'settled').exists())

    def start_owned_unit(self, unit, root, policy):
        command = ['systemd-run', '--user', '--quiet', '--unit=' + unit]
        for key, default in [('TimeoutStopSec', '15'), ('KillMode', 'control-group'),
                             ('SendSIGKILL', 'yes')]:
            command.append('--property=' + key + '=' + policy.get(key, default))
        command.extend([sys.executable, str(Path(__file__).with_name('service_retirement_fixture.py')),
                        str(root)])
        run(*command)

    def assert_retained(self, unit, root):
        state = run('systemctl', '--user', 'show', unit,
                    '--property=ActiveState', '--value').stdout.strip()
        self.assertEqual(state, 'deactivating', 'Original cleanup owner was killed')
        self.assertFalse((root / 'settled').exists())
        self.assertFalse((root / 'child-signalled').exists(),
                         'Service manager bypassed the parent native owner')


if __name__ == '__main__':
    unittest.main()
