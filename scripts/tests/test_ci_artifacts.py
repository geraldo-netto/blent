"""T1360: execute the CI release selection against real Cargo feature gates."""
from pathlib import Path
import os
import re
import shlex
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


def package_fixture(root, package):
    original = tomllib.loads((ROOT / package / 'Cargo.toml').read_text())
    name = original['package']['name']
    directory = root / package
    directory.mkdir()
    manifest = [f'[package]\nname = "{name}"\nversion = "0.0.0"\nedition = "2021"', '[features]']
    manifest.extend(f'{feature} = []' for feature in original.get('features', {}))
    dependency = original.get('target', {}).get('cfg(target_os = "linux")', {}).get('dependencies', {}).get('blent')
    if dependency:
        manifest.append('[dependencies]\nblent = { path = "../host", features = ' + repr(dependency['features']) + ' }')
    (directory / 'lib.rs').write_text('')
    manifest.append('[lib]\npath = "lib.rs"')
    targets = [*original.get('bin', []), {'name': name}]
    for target in targets:
        binary = target['name']
        manifest.append(f'[[bin]]\nname = "{binary}"\npath = "{binary}.rs"')
        manifest.append('required-features = ' + repr(target.get('required-features', [])))
        (directory / f'{binary}.rs').write_text('fn main() { println!("fresh-' + binary + '"); }\n')
    (directory / 'Cargo.toml').write_text('\n'.join(manifest) + '\n')


class CiArtifactsTest(unittest.TestCase):
    def test_t1360_release_build_produces_every_advertised_rust_binary(self):
        workflow = (ROOT / '.github/workflows/build.yml').read_text()
        command = re.search(r'name: Build daemon and GUI\n\s+run: (.+)', workflow).group(1)
        binaries = set(re.findall(r'target/release/(blent[\w-]*)', workflow)) - {'blent-usb-io'}
        self.assertEqual(binaries, {'blent', 'blent-gui', 'blent-audio', 'blent-usb-broker'})
        for stale in [False, True]:
            with self.subTest(stale=stale), tempfile.TemporaryDirectory() as temporary:
                self.verify_build(Path(temporary), command, binaries, stale)

    def verify_build(self, root, command, binaries, stale):
        (root / 'Cargo.toml').write_text('[workspace]\nmembers = ["host", "gui"]\nresolver = "2"\n')
        for package in ['host', 'gui']:
            package_fixture(root, package)
        output = root / 'target/release'
        output.mkdir(parents=True)
        if stale:
            (output / 'blent-usb-broker').write_text('stale broker')
        env = dict(os.environ, CARGO_TARGET_DIR=str(root / 'target'), CARGO_NET_OFFLINE='true')
        built = subprocess.run(shlex.split(command), cwd=root, env=env, text=True, capture_output=True)
        self.assertEqual(built.returncode, 0, built.stderr)
        for binary in sorted(binaries):
            path = output / binary
            self.assertTrue(path.is_file(), f'T1360: CI omitted {binary}')
            self.assertTrue(os.access(path, os.X_OK), f'T1360: reused stale {binary}')
            self.assertEqual(subprocess.check_output([str(path)], text=True), f'fresh-{binary}\n')


if __name__ == '__main__':
    unittest.main()
