#!/usr/bin/python3
"""T1440 actual-entry fixture; PID namespace teardown contains all descendants."""
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time


def configure(root, case):
    shutil.copyfile(Path(__file__).with_name('fake_adb.py'), root / 'adb')
    (root / 'adb').chmod(0o700)
    config = root / 'config/blent/config.toml'
    config.parent.mkdir(parents=True)
    if case == 'off':
        config.write_text('wifi_address = "192.0.2.74:5555"\n')
        (root / 'route').write_text('192.0.2.74:5555')
    elif case == 'invalid':
        config.write_text('not valid toml !!!')
    return dict(os.environ, HOME=str(root), XDG_CONFIG_HOME=str(root / 'config'),
                XDG_RUNTIME_DIR=str(root), XDG_STATE_HOME='private-no-broker',
                PATH=str(root), RUST_LOG='off', BLENT_TEST_ROOT=str(root), CASE=case)


def await_started(root, child, operation):
    until = time.monotonic() + 15
    while not (root / (operation + '-started')).exists():
        assert child.poll() is None, f'{operation}: child exited {child.poll()}'
        assert time.monotonic() < until, operation
        time.sleep(0.01)


def assert_retained(child, operation):
    try:
        returned = child.wait(timeout=0.2)
    except subprocess.TimeoutExpired:
        return
    raise AssertionError(f'T1440 CLI exited {returned} before original {operation} settled')


def interrupt(root, child, case, number):
    first = 'disconnect' if case == 'off' else 'connect'
    await_started(root, child, first)
    child.send_signal(number)
    assert_retained(child, first)
    if case != 'off':
        (root / 'connect-release').touch()
        await_started(root, child, 'disconnect')
        assert_retained(child, 'disconnect')
    (root / 'disconnect-release').touch()


def finish(root, child, case, number):
    result = child.wait(timeout=15)
    expected = 1 if number or case in ['rollback', 'invalid'] else 0
    assert result == expected, (case, number, result, (root / 'output').read_text())
    retained = case == 'setup' and number == 0
    assert (root / 'route').exists() == retained, (case, number)
    config = root / 'config/blent/config.toml'
    saved = config.exists() and '192.0.2.74:5555' in config.read_text()
    assert saved == retained, (case, number, 'saved config')
    if number:
        assert 'interrupted' in (root / 'output').read_text().lower()
    if case == 'invalid':
        assert 'config' in (root / 'output').read_text().lower()
    if case == 'rollback':
        assert 'ADB did not confirm network connection' in (root / 'output').read_text()
    print(f'PASS case={case} signal={number} exit={result}', flush=True)


def run(binary, case, number):
    with tempfile.TemporaryDirectory(prefix='blent-t1440-') as directory:
        root = Path(directory)
        env = configure(root, case)
        if not number:
            (root / 'connect-release').touch()
            (root / 'disconnect-release').touch()
        args = [binary, 'wifi'] + (['--off'] if case == 'off' else [])
        with (root / 'output').open('w') as output:
            child = subprocess.Popen(args, env=env, stdin=subprocess.DEVNULL,
                                     stdout=output, stderr=output)
            try:
                if number:
                    interrupt(root, child, case, number)
                finish(root, child, case, number)
            finally:
                (root / 'connect-release').touch()
                (root / 'disconnect-release').touch()
                if child.poll() is None:
                    child.kill()
                child.wait()


if __name__ == '__main__':
    for selected in [signal.SIGINT, signal.SIGTERM]:
        for scenario in ['setup', 'off']:
            run(sys.argv[1], scenario, selected)
    for scenario in ['setup', 'off', 'rollback', 'invalid']:
        run(sys.argv[1], scenario, 0)
