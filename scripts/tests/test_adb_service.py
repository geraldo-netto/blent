"""T1465: shared ADB must never become part of Blent's retirement group."""
import os
from pathlib import Path
import socket
import select
import shlex
import tempfile
import time
import uuid
import subprocess
import unittest

from test_service_retirement import ROOT, TEMPLATES, run, service_policy, await_file
from adb_service_fixture import reply


class AdbServiceTest(unittest.TestCase):
    def test_t1465_all_units_separate_server_start_and_client_connection(self):
        for template in TEMPLATES:
            with self.subTest(template=template):
                policy = service_policy(template)
                self.assertEqual(policy.get('Environment'),
                                 'ADB_SERVER_SOCKET=tcp:127.0.0.1:5037')
                self.assertIn('systemd-run --user --scope --quiet --', policy['ExecStartPre'])
                self.assertIn('--blent-start-adb-server', policy['ExecStartPre'])
                rendered = run('bash', str(ROOT / 'scripts/write-systemd-service.sh'),
                               '/tmp/Blent fixture', str(ROOT / template)).stdout
                self.assertIn('"/tmp/Blent fixture" --blent-start-adb-server', rendered)
                self.assertIn('SendSIGKILL=no', rendered)

    def test_t1465_native_adb_client_does_not_spawn_on_server_loss(self):
        # Reserved loopback port: this cannot contact or alter the real ADB server.
        with socket.socket() as reserved:
            reserved.bind(('127.0.0.1', 0))
            port = reserved.getsockname()[1]
            result = subprocess.run(['adb', 'devices'], capture_output=True, text=True,
                                    env=dict(os.environ, ADB_SERVER_SOCKET=f'tcp:127.0.0.1:{port}'))
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn('daemon started successfully', result.stderr)
            self.assertNotIn('starting now', result.stderr)
            self.assertIn('cannot connect', result.stderr)

    def test_t1465_native_scope_survives_daemon_stop_and_restart(self):
        unit = 'blent-t1465-' + uuid.uuid4().hex + '.service'
        units = Path(os.environ['XDG_RUNTIME_DIR']) / 'systemd/user'
        units.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='blent-t1465-') as directory:
            root = Path(directory)
            launcher = root / 'launcher'
            fixture = Path(__file__).with_name('adb_service_fixture.py')
            launcher.write_text('#!/bin/sh\nexec /usr/bin/python3 ' +
                                shlex.join([str(fixture), str(root)]) + ' "$@"\n')
            launcher.chmod(0o700)
            text = run('bash', str(ROOT / 'scripts/write-systemd-service.sh'),
                       str(launcher), str(ROOT / TEMPLATES[0])).stdout
            (units / unit).write_text(text)
            run('systemctl', '--user', 'daemon-reload')
            try:
                self.exercise_native(unit, root)
            finally:
                self.stop_private_server(root)
                subprocess.run(['systemctl', '--user', 'stop', unit], capture_output=True)
                (units / unit).unlink()
                run('systemctl', '--user', 'daemon-reload')
                subprocess.run(['systemctl', '--user', 'reset-failed', unit], capture_output=True)

    def exercise_native(self, unit, root):
        for _ in range(2):
            run('systemctl', '--user', 'start', unit)
            await_file(root / 'daemon-pid')
            await_file(root / 'server-pid')
            server_pid = (root / 'server-pid').read_text()
            group = Path('/proc', server_pid, 'cgroup').read_text()
            self.assertNotIn(unit, group)
            self.assertIn('.scope', group)
            original = (root / 'daemon-pid').read_text()
            (root / 'daemon-pid').unlink()
            run('systemctl', '--user', 'restart', unit)
            await_file(root / 'daemon-pid')
            self.assertNotEqual(original, (root / 'daemon-pid').read_text())
            self.assertEqual(server_pid, (root / 'server-pid').read_text())
            run('systemctl', '--user', 'stop', unit)
            self.assertEqual(reply(root, b'ping'), b'owned-t1465')
            (root / 'daemon-pid').unlink()
            self.stop_private_server(root)

    def stop_private_server(self, root):
        if not (root / 'server-pid').exists():
            return
        pidfd = os.pidfd_open(int((root / 'server-pid').read_text()))
        try:
            self.assertEqual(reply(root, b'stop'), b'owned-t1465')
            self.assertTrue(select.select([pidfd], [], [], 10)[0],
                            'T1465 private server failed to exit after its stop receipt')
        finally:
            os.close(pidfd)


if __name__ == '__main__':
    unittest.main()
