#!/usr/bin/python3
"""T1423: namespace-only startup barriers with exclusively fake ADB effects."""
import errno
import json
import os
from pathlib import Path
import select
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time


def port():
    with socket.socket() as held:
        held.bind(('127.0.0.1', 0))
        return held.getsockname()[1]


def configure(root):
    config = root / 'config/blent/config.toml'
    config.parent.mkdir(parents=True)
    config.write_text(f'''encoder = "libx264"
encoding_gpu = "software"
input_touch = false
input_pen = false
input_pointer = false
check_updates = false
auto_launch_app = false
video_port = {port()}
input_port = {port()}
''')
    shutil.copyfile(Path(__file__).with_name('fake_adb.py'), root / 'adb')
    (root / 'adb').chmod(0o700)
    env = dict(os.environ, HOME=str(root), XDG_CONFIG_HOME=str(root / 'config'),
               XDG_RUNTIME_DIR=str(root), XDG_STATE_HOME='t1423-no-native-broker',
               PATH=str(root), DBUS_SESSION_BUS_ADDRESS='unix:path=/missing-test-bus',
               DBUS_SYSTEM_BUS_ADDRESS='unix:path=/missing-test-system-bus',
               RUST_LOG='off', BLENT_REVIEW_ROOT=str(root))
    for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'BLENT_FAKE_TABLET', 'XAUTHORITY']:
        env.pop(key, None)
    return env, config


def await_condition(child, predicate, description):
    end = time.monotonic() + 15
    while child.poll() is None and time.monotonic() < end:
        if predicate():
            return
        time.sleep(0.01)
    raise AssertionError(f'{description}: child={child.poll()}')


def routes(root):
    path = root / 'routes.json'
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError:
        return {}  # A fake command may be publishing; only stable counts pass.


def caught(child, number):
    fields = dict(line.split(':', 1) for line in Path(f'/proc/{child.pid}/status').read_text().splitlines())
    return bool(int(fields['SigCgt'].strip(), 16) & (1 << (number - 1)))


def full_pipe():
    read, write = os.pipe()
    os.set_blocking(write, False)
    try:
        while True:
            os.write(write, b'x' * 4096)
    except BlockingIOError:
        pass
    os.set_blocking(write, True)
    os.set_blocking(read, False)
    return read, write


def drain_once(read):
    if select.select([read], [], [], 0.05)[0]:
        return os.read(read, 65536)
    return b''


def await_banner(child, read):
    output = bytearray()
    def printed():
        output.extend(drain_once(read))
        return b'Otherwise, run:' in output
    await_condition(child, printed, 'running banner')


def settle(child, read):
    end = time.monotonic() + 15
    while child.poll() is None and time.monotonic() < end:
        drain_once(read)
    return child.wait(timeout=1)


def reap(child):
    if child.poll() is None:
        child.kill()
    child.wait()


def banner_case(binary, number, ready):
    with tempfile.TemporaryDirectory(prefix='blent-t1423-') as directory:
        root = Path(directory)
        env, _ = configure(root)
        read, write = full_pipe()
        with (root / 'stderr').open('w') as errors:
            child = subprocess.Popen([binary, '--helper', '/bin/true', 'start'], env=env,
                                     stdin=subprocess.DEVNULL, stdout=write, stderr=errors)
        os.close(write)
        try:
            await_condition(child, lambda: len(routes(root)) == 2, 'two fake routes')
            if ready:
                await_banner(child, read)
                await_condition(child, lambda: caught(child, number), 'post-startup handler')
            installed = caught(child, number)
            child.send_signal(number)
            code = settle(child, read)
            result = dict(case='banner', signal=number, ready=ready, installed=installed,
                          exit=code, routes=len(routes(root)))
            print(json.dumps(result), flush=True)
            assert result == dict(case='banner', signal=number, ready=ready,
                                  installed=True, exit=0, routes=0), (result, (root / 'stderr').read_text())
        finally:
            reap(child)
            os.close(read)


def open_writer(child, config):
    writer = []
    def opened():
        try:
            writer.append(os.open(config, os.O_WRONLY | os.O_NONBLOCK))
            return True
        except OSError as error:
            if error.errno != errno.ENXIO:
                raise
            return False
    await_condition(child, opened, 'config reader')
    return writer[0]


def config_case(binary, invalid):
    with tempfile.TemporaryDirectory(prefix='blent-t1423-early-') as directory:
        root = Path(directory)
        env, config = configure(root)
        content = config.read_bytes()
        config.unlink()
        os.mkfifo(config)
        args = [binary, '--helper', '/bin/true']
        if invalid:
            args += ['--pen-only']
        args += ['start']
        with (root / 'stderr').open('w') as errors:
            child = subprocess.Popen(args, env=env, stdin=subprocess.DEVNULL,
                                     stdout=subprocess.DEVNULL, stderr=errors)
        try:
            with os.fdopen(open_writer(child, config), 'wb', buffering=0) as writer:
                installed = caught(child, signal.SIGTERM)
                child.send_signal(signal.SIGTERM)
                time.sleep(0.1)  # Let the independent observer handle the request at this barrier.
                try:
                    writer.write(content)
                except BrokenPipeError:
                    pass  # Preserve the pre-fix signal-termination evidence.
            code = child.wait(timeout=15)
            result = dict(case='config', invalid=invalid, installed=installed,
                          exit=code, adb_started=(root / 'adb-calls.txt').exists())
            print(json.dumps(result), flush=True)
            assert result == dict(case='config', invalid=invalid, installed=True,
                                  exit=int(invalid), adb_started=False), (result, (root / 'stderr').read_text())
            if invalid:
                assert 'graphics-tablet' in (root / 'stderr').read_text().lower()
        finally:
            reap(child)


def main():
    assert os.getpid() == 1, 'Must run in isolated PID namespace'
    subprocess.run(['/bin/mount', '-t', 'tmpfs', 'tmpfs', '/dev'], check=True)
    Path('/dev/null').touch()
    binary = str(Path(sys.argv[1]).resolve())
    for number in [signal.SIGTERM, signal.SIGINT]:
        for ready in [False, True]:
            banner_case(binary, number, ready)
    for invalid in [False, True]:
        config_case(binary, invalid)


if __name__ == '__main__':
    main()
