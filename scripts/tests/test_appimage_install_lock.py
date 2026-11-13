"""T1407: a destination has one installation owner through final registration."""
import os
from pathlib import Path
import shutil
import subprocess
import time
import unittest

import test_appimage


class AppImageInstallLockTest(unittest.TestCase):
    def setUp(self):
        self.fixture = test_appimage.AppImageTest()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.fixture.installer_fixture()
        self.fixture.write(self.fixture.app / 'version', 'first')

    def wait_for_publication(self, marker, process):
        deadline = time.monotonic() + 5
        while not marker.exists():
            self.assertIsNone(process.poll(), 'T1407: installer exited before publication')
            self.assertLess(time.monotonic(), deadline, 'T1407: publication gate not reached')
            time.sleep(0.01)

    def prepare_gate(self):
        root = self.fixture.root
        script = '''#!/bin/bash
if [[ -n ${T1407_GATE:-} && ${@: -2:1} == */.appdir.* ]]; then
    touch "$T1407_GATE/reached"
    while [[ -e $T1407_GATE/hold ]]; do sleep 0.01; done
fi
exec /usr/bin/mv "$@"
'''
        self.fixture.write(self.fixture.app / 'usr/bin/mv', script)
        gate = root / 'gate'
        gate.mkdir()
        (gate / 'hold').touch()
        second = root / 'second.AppDir'
        shutil.copytree(self.fixture.app, second, symlinks=True)
        (second / 'version').write_text('second')
        return gate, second

    def concurrent_install(self, gate, second):
        environment = dict(self.fixture.env, T1407_GATE=str(gate))
        first = subprocess.Popen([str(self.fixture.app / 'AppRun'), '--install-user'],
                                 env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            self.wait_for_publication(gate / 'reached', first)
            result = subprocess.run([str(second / 'AppRun'), '--install-user'],
                                    env=self.fixture.env, capture_output=True, text=True, timeout=5)
        finally:
            (gate / 'hold').unlink(missing_ok=True)
            output, errors = first.communicate(timeout=10)
        self.assertEqual(first.returncode, 0, output + errors)
        return result

    def test_t1407_competing_install_cannot_publish_or_register(self):
        gate, second = self.prepare_gate()
        result = self.concurrent_install(gate, second)
        self.assertNotEqual(result.returncode, 0, 'T1407: competing installer claimed success')
        destination = Path(self.fixture.env['XDG_DATA_HOME']) / 'blent/appimage'
        installed = destination / 'Blent.AppDir'
        self.assertEqual((installed / 'version').read_text(), 'first')
        self.assertEqual(list(installed.glob('.appdir.*')), [])
        self.assertEqual(list(destination.glob('.appdir.*')), [])
        self.assertFalse((destination / 'Blent.AppDir.previous').exists())
        retry = subprocess.run([str(second / 'AppRun'), '--install-user'],
                               env=self.fixture.env, capture_output=True, text=True, timeout=10)
        self.assertEqual(retry.returncode, 0, retry.stderr)
        self.assertEqual((installed / 'version').read_text(), 'second')
