"""T102: local distributions need an APK and a loadable bundled helper."""
from pathlib import Path
import os
import shutil
import shlex
import subprocess
import tempfile
import unittest

import test_notices
import release_signing_fixture


def tree_contents(root):
    entries = {}
    for path in root.rglob('*'):
        mode = path.lstat().st_mode
        if path.is_symlink():
            content = ('link', os.readlink(path))
        elif path.is_file():
            content = ('file', path.read_bytes())
        else:
            content = ('directory', None)
        entries[str(path.relative_to(root))] = (mode, content)
    return entries


class DistributionTest(unittest.TestCase):
    def assert_apk_gate(self, root, result, mode, version):
        # T1301: an earlier native-build failure cannot pass an APK rejection case.
        self.assertTrue((root / 'apk-gate-reached').is_file(), result.stdout + result.stderr)
        if mode == 'success':
            return
        self.assertFalse((root / f'dist/blent-{version}-linux-x86_64.tar.gz').exists())
        if mode == 'failed-apk':
            self.assertIn('Error 42', result.stderr)
        else:
            # SDK reports are fixtures; the real bundle copy rejects absence.
            self.assertIn('cp: cannot stat', result.stderr)
            self.assertIn('app-release.apk', result.stderr)

    def compare_shared_bundle(self, root, local, library):
        staged = root / 'shared bundle'
        subprocess.run(['bash', 'scripts/stage-linux-bundle.sh', 'target/release',
                        'host/evdi/evdi_helper', str(library), str(staged)], cwd=root, check=True)
        expected = tree_contents(local)
        # APK production belongs to the caller; CI Linux package fixtures omit it.
        expected.pop('blent.apk')
        self.assertEqual(tree_contents(staged), expected, 'T379: bundle layouts diverged')
        self.assertEqual(os.readlink(staged / 'bin/libevdi.so.1'), 'libevdi.so.1.15.0')
        self.assertFalse((staged / 'bin/libevdi.so.1.15.0').is_symlink())
        test_notices.NoticeTest().verify_docs(staged)

    def test_t102_apk_failure_and_bundled_helper_loading(self):
        for mode in ['failed-apk', 'missing-apk', 'success']:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(prefix='blent-dist-') as tmp:
                root = Path(tmp)
                version = test_notices.NoticeTest().copy_sources(root)
                def write(name, body, executable=False):
                    path = root / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(body)
                    if executable:
                        path.chmod(0o755)
                    return path
                write('target/release/blent', '#!/bin/sh\nexit 0\n', True)
                write('target/release/blent-gui', '#!/bin/sh\nexit 0\n', True)
                write('target/release/blent-audio', '#!/bin/sh\nexit 0\n', True)
                write('target/release/blent-usb-broker', '#!/bin/sh\nexit 0\n', True)
                # T1301: independent USB build fixture honors the real output contract.
                write('host/accessory/Makefile', '$(OUTPUT): helper.c\n\t$(CC) helper.c -o "$(OUTPUT)"\n')
                write('host/accessory/helper.c', 'int main(void)\n{\n    return 0;\n}\n')
                write('library.c', 'int blent_distribution_probe(void) { return 0; }\n')
                write('host/evdi/evdi_helper.c', 'extern int blent_distribution_probe(void); int main(void) { return blent_distribution_probe(); }\n')
                # T380: stub the other C units too; this fixture checks loader
                # paths and packaging. evdi_modules tests the real module link.
                # T832: retain cursor_overlay too; Makefile links it into the helper.
                for module in ['conversion', 'frame_exchange', 'fifo_writer', 'capture',
                               'capture_activity', 'capture_control', 'cursor_overlay',
                               'raw_ring', 'writer', 'idle_control']:
                    write(f'host/evdi/{module}.c', '/* distribution fixture */\n')
                # T642: model an installed compiler library that precedes LIBRARY_PATH.
                competing = root / 'compiler libraries'
                competing.mkdir()
                write('competing.c', 'int unrelated_evdi(void) { return 0; }\n')
                subprocess.run(['cc', '-shared', '-fPIC', 'competing.c', '-o', str(competing / 'libevdi.so')], cwd=root, check=True)
                compiler = ['gcc', '-B', str(competing) + '/']
                library = root / "library's space/libevdi.so.1.15.0"
                library.parent.mkdir()
                compiler.extend(['-L', str(library.parent)])
                subprocess.run(['cc', '-shared', '-fPIC', '-Wl,-soname,libevdi.so.1', 'library.c', '-o', str(library)], cwd=root, check=True)
                (library.parent / 'libevdi.so').symlink_to(library.name)
                (library.parent / 'libevdi.so.1').symlink_to(library.name)
                if mode == 'success':
                    write('android/app/build/outputs/apk/release/app-release.apk', 'apk')
                    library.rename(library.parent / 'actual.so')
                    library.symlink_to('actual.so')
                write('android/gradlew', '#!/bin/sh\ntouch apk-gate-reached\nexit ' + ('42' if mode == 'failed-apk' else '0') + '\n', True)
                # T505: retain the real release gate with offline SDK reports.
                release_signing_fixture.install_tools(root / "bin")
                env = dict(os.environ, LIBRARY_PATH=str(library.parent),
                           PATH=f'{root}/bin:{os.environ["PATH"]}')
                result = subprocess.run(['make', 'dist-local', 'CARGO=true', f'LIBEVDI={library}', 'CC=' + shlex.join(compiler)], cwd=root, env=env, capture_output=True, text=True)
                self.assert_apk_gate(root, result, mode, version)
                if mode != 'success':
                    self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                else:
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    # Extract and remove build-time library search paths entirely.
                    artifact = root / 'extracted'
                    artifact.mkdir()
                    subprocess.run(['tar', '-xf', f'dist/blent-{version}-linux-x86_64.tar.gz', '-C', str(artifact)], cwd=root, check=True)
                    self.compare_shared_bundle(root, artifact / f'blent-{version}', library)
                    shutil.rmtree(library.parent)
                    env.pop('LIBRARY_PATH')
                    env.pop('LD_LIBRARY_PATH', None)
                    helper = artifact / f'blent-{version}/bin/evdi_helper'
                    loaded = subprocess.run([str(helper)], env=env, capture_output=True, text=True)
                    self.assertEqual(loaded.returncode, 0, loaded.stderr)
                    self.assertTrue((artifact / f'blent-{version}/blent.apk').is_file())
                    # T129: the local tarball carries the same notices and working links.
                    test_notices.NoticeTest().verify_docs(artifact / f'blent-{version}')

    def test_t379_missing_bundle_inputs_cannot_produce_an_archive(self):
        inputs = ['target/release/blent', 'target/release/blent-gui',
                  'target/release/blent-audio', 'target/release/blent-usb-broker',
                  'target/release/blent-usb-io',
                  'host/evdi/evdi_helper', 'library/libevdi.so.1.15.0']
        for missing in inputs:
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                version = test_notices.NoticeTest().copy_sources(root)
                for name in inputs:
                    path = root / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text('fixture')
                    path.chmod(0o751)
                accessory = root / 'host/accessory'
                accessory.mkdir()
                (accessory / 'Makefile').write_text('.PHONY: all\nall:\n\ttrue\n')
                commands = [
                    ['bash', 'scripts/stage-linux-bundle.sh', 'target/release',
                     'host/evdi/evdi_helper', 'library/libevdi.so.1.15.0', 'staged'],
                    ['make', 'dist-local', 'CC=true', 'CARGO=true',
                     'LIBEVDI=library/libevdi.so.1.15.0'],
                ]
                # T1301: prove staging succeeds before removing exactly one input.
                baseline = subprocess.run(commands[0], cwd=root, capture_output=True, text=True)
                self.assertEqual(baseline.returncode, 0, baseline.stdout + baseline.stderr)
                (root / missing).unlink()
                for command in commands:
                    result = subprocess.run(command, cwd=root, capture_output=True, text=True)
                    self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertIn(missing, result.stderr)
                self.assertFalse((root / f'dist/blent-{version}-linux-x86_64.tar.gz').exists())


if __name__ == '__main__':
    unittest.main()
