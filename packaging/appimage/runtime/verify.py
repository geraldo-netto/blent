#!/usr/bin/env python3
"""T1622: execute the actual runtime against a tiny private SquashFS image."""
import ctypes
import os
from pathlib import Path
import signal
import resource
import subprocess
import sys
import tempfile
import time


def build_image(runtime, root):
    app = root / 'AppDir'
    app.mkdir()
    (app / 'bundled-tool').write_text('original bundled executable\n')
    (app / 'AppRun').write_text('''#!/bin/sh
trap 'printf signalled > "$BLENT_PROBE_ROOT/signalled"' TERM INT HUP
printf '%s\\n' "$APPDIR" > "$BLENT_APPDIR_RECORD"
if [ "$1" = hold ]; then
    while [ ! -e "$BLENT_PROBE_ROOT/release" ]; do /bin/sleep 0.02; done
fi
''')
    (app / 'AppRun').chmod(0o700)
    squash = root / 'fixture.squashfs'
    subprocess.run(['mksquashfs', str(app), str(squash), '-noappend', '-comp', 'gzip',
                    '-all-root', '-processors', '2', '-quiet'], check=True, stdout=subprocess.DEVNULL)
    image = root / 'fixture.AppImage'
    image.write_bytes(runtime.read_bytes() + squash.read_bytes())
    image.chmod(0o700)
    return image


def ready(child, record):
    end = time.monotonic() + 5
    while not record.is_file() or record.stat().st_size == 0:
        if child.poll() is not None:
            raise AssertionError(f'runtime exited before AppRun readiness: {child.returncode}')
        if time.monotonic() >= end:
            raise AssertionError('AppRun readiness protocol expired')
        time.sleep(0.01)
    return Path(record.read_text().strip())


def invocation(image, env, argument):
    flags = [] if 'APPIMAGE_EXTRACT_AND_RUN' in env else ['--appimage-extract-and-run']
    return [str(image), *flags, argument]


def check_sibling(image, env, original):
    record = Path(env['BLENT_PROBE_ROOT']) / 'sibling'
    subprocess.run(invocation(image, env, 'probe'),
                   env=dict(env, BLENT_APPDIR_RECORD=str(record)), check=True)
    sibling = Path(record.read_text().strip())
    assert (original / 'bundled-tool').is_file(), 'T1622 sibling deleted the live bundled tool'
    assert sibling != original, 'T1622 invocations share an extraction directory'
    assert not sibling.exists(), 'T1622 settled sibling retained its extraction'


def check_signal(child, env, original):
    child.send_signal(signal.SIGTERM)
    ready(child, Path(env['BLENT_PROBE_ROOT']) / 'signalled')
    assert child.poll() is None, 'T1622 runtime exited before its original child'
    assert (original / 'bundled-tool').is_file(), 'T1622 signal removed live bundled files'


def retire(child, root):
    (root / 'release').touch()
    try:
        child.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGKILL)
        child.wait()
        raise AssertionError('runtime failed original-child retirement protocol')
    finally:
        # This fixture is a subreaper: reap any orphaned original descendants too.
        while True:
            try:
                os.waitpid(-1, 0)
            except ChildProcessError:
                break


def verify(runtime, environment=False):
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    assert ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0
    with tempfile.TemporaryDirectory(prefix='blent-t1622-runtime-') as directory:
        root = Path(directory)
        temporary = root / 'tmp'
        temporary.mkdir(mode=0o700)
        image = build_image(runtime, root)
        env = dict(os.environ, TMPDIR=str(temporary), BLENT_PROBE_ROOT=str(root),
                   BLENT_APPDIR_RECORD=str(root / 'holder'))
        env.pop('APPIMAGE_EXTRACT_AND_RUN', None)
        if environment:
            env['APPIMAGE_EXTRACT_AND_RUN'] = '1'
        env.pop('NO_CLEANUP', None)
        child = subprocess.Popen(invocation(image, env, 'hold'),
                                 env=env, start_new_session=True)
        try:
            original = ready(child, root / 'holder')
            check_sibling(image, env, original)
            check_signal(child, env, original)
        finally:
            retire(child, root)
        assert child.returncode == 0, 'original AppRun status was lost'
        assert not original.exists(), 'T1622 settled original retained its extraction'
        assert not list(temporary.iterdir()), 'T1622 runtime left extraction resources'


def verify_paths(runtime):
    """T1639: native entry rejects oversized environment/argv/portable paths."""
    with tempfile.TemporaryDirectory(prefix='blent-t1639-') as directory:
        root = Path(directory)
        image = build_image(runtime, root)
        for variable in ('TMPDIR', 'TARGET_APPIMAGE'):
            result = subprocess.run([str(image), '--appimage-version'],
                                    env=dict(os.environ, **{variable: '/' + 'x' * 4096}), capture_output=True)
            assert result.returncode == 127 and b'AppImage path' in result.stderr, variable
        result = subprocess.run(['x' * 4096, '--appimage-version'], executable=str(image), capture_output=True)
        assert result.returncode == 127 and b'AppImage path' in result.stderr, 'oversized argv[0]'
        parent = root
        while len(str(parent)) < 3900:
            parent /= 'd' * 200
            parent.mkdir()
        name = 'i' * (4092 - len(str(parent)) - 1)
        long_image = parent / name
        long_image.write_bytes(image.read_bytes())
        long_image.chmod(0o700)
        result = subprocess.run([str(long_image), '--appimage-portable-home'], capture_output=True)
        assert result.returncode == 127 and b'AppImage path' in result.stderr, 'portable suffix overflow'


if __name__ == '__main__':
    verify(Path(sys.argv[1]).resolve())
    verify(Path(sys.argv[1]).resolve(), environment=True)
    verify_paths(Path(sys.argv[1]).resolve())
