"""Authoritative Cargo invocations for the native Linux release (T1281)."""
from pathlib import Path
import subprocess

TARGET = 'x86_64-unknown-linux-gnu'
BUILD_UNITS = (('host/Cargo.toml', ('native-audio', 'accessory-transport')),
               ('gui/Cargo.toml', ()))


def build_options(unit):
    manifest, features = unit
    options = ['--manifest-path', manifest]
    if features:
        options.extend(['--features', ','.join(features)])
    return options


def build_release(repo):
    for unit in BUILD_UNITS:
        subprocess.run(['cargo', 'build', '--release', '--locked', *build_options(unit)],
                       cwd=repo, check=True)


if __name__ == '__main__':
    build_release(Path(__file__).resolve().parents[2])
