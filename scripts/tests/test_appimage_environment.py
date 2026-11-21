"""T1623: inherited AppImage variables belong to their original application."""
import os
from pathlib import Path
import subprocess
import unittest

import test_appimage


class AppImageEnvironmentTest(unittest.TestCase):
    def setUp(self):
        self.fixture = test_appimage.AppImageTest()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.outer = self.fixture.write(self.fixture.root / 'foreign.AppImage',
                                        '#!/bin/sh\nprintf "foreign application\\n"\n')

    def foreign_environments(self):
        foreign = self.fixture.root / 'foreign.AppDir'
        foreign.mkdir()
        for directory in (None, '', str(foreign), str(foreign / 'missing')):
            env = dict(self.fixture.env, APPIMAGE=str(self.outer))
            env.pop('APPDIR', None)
            if directory is not None:
                env['APPDIR'] = directory
            yield env

    def test_t1623_extracted_launch_ignores_foreign_outer_image(self):
        self.fixture.write(self.fixture.app / 'usr/bin/blent',
                           '#!/bin/sh\nprintf "%s\\n" "$BLENT_APPIMAGE_LAUNCHER" "${APPIMAGE-unset}" "$APPDIR"\n')
        for env in self.foreign_environments():
            with self.subTest(directory=env.get('APPDIR')):
                result = self.fixture.invoke('status', env=env)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.splitlines(),
                                 [str(self.fixture.app / 'AppRun'), 'unset', str(self.fixture.app)])

    def install(self, env, direct):
        if not direct:
            return self.fixture.invoke('--install-user', env=env)
        env = dict(env, PATH=str(self.fixture.app / 'usr/bin') + os.pathsep + env['PATH'])
        script = self.fixture.app / 'usr/share/blent/install-appimage.sh'
        return subprocess.run(['/bin/bash', str(script)], env=env,
                              capture_output=True, text=True, timeout=20)

    def verify_foreign_install(self, direct):
        self.fixture.installer_fixture()
        for index, env in enumerate(self.foreign_environments()):
            with self.subTest(directory=env.get('APPDIR')):
                env['XDG_DATA_HOME'] = str(self.fixture.root / f'data-{index}')
                result = self.install(env, direct)
                self.assertEqual(result.returncode, 0, result.stderr)
                destination = Path(env['XDG_DATA_HOME']) / 'blent/appimage'
                self.assertTrue((destination / 'Blent.AppDir/AppRun').is_file())
                self.assertFalse((destination / 'Blent.AppImage').exists())
                self.assertEqual(self.outer.read_text(), '#!/bin/sh\nprintf "foreign application\\n"\n')

    def test_t1623_registration_ignores_foreign_outer_image(self):
        self.verify_foreign_install(False)

    def test_t1623_direct_installer_ignores_foreign_outer_image(self):
        self.verify_foreign_install(True)

    def test_t1623_matching_directory_alias_retains_outer_image(self):
        self.fixture.installer_fixture()
        alias = self.fixture.root / 'directory alias'
        alias.symlink_to(self.fixture.app, target_is_directory=True)
        env = dict(self.fixture.env, APPDIR=str(alias), APPIMAGE=str(self.outer))
        for direct in (False, True):
            with self.subTest(direct=direct):
                result = self.install(env, direct)
                self.assertEqual(result.returncode, 0, result.stderr)
                installed = Path(env['XDG_DATA_HOME']) / 'blent/appimage/Blent.AppImage'
                self.assertEqual(installed.read_bytes(), self.outer.read_bytes())
