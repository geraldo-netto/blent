#!/usr/bin/python3
"""Run only inside private user/mount/PID namespaces; mask native device nodes."""
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
pen_only = false
video_port = {port()}
input_port = {port()}
''')
    shutil.copyfile(Path(__file__).with_name('fake_adb.py'), root / 'adb')
    (root / 'adb').chmod(0o700)
    env = dict(os.environ, HOME=str(root), XDG_CONFIG_HOME=str(root / 'config'),
               XDG_RUNTIME_DIR=str(root), XDG_STATE_HOME=str(root / 'state'),
               PATH=str(root), DBUS_SESSION_BUS_ADDRESS='unix:path=/missing-review-bus',
               DBUS_SYSTEM_BUS_ADDRESS='unix:path=/missing-review-system-bus',
               RUST_LOG='off', BLENT_REVIEW_ROOT=str(root))
    for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'BLENT_FAKE_TABLET', 'XAUTHORITY']:
        env.pop(key, None)
    return env


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


def await_routes(child, root):
    end = time.monotonic() + 10
    path = root / 'routes.json'
    while child.poll() is None and time.monotonic() < end:
        if path.exists() and len(json.loads(path.read_text())) == 2:
            return
        time.sleep(0.02)
    raise RuntimeError(f'No two fake routes: child={child.poll()} stderr={(root / "stderr").read_text()} calls={list(root.iterdir())}')


def caught(child):
    fields = dict(line.split(':', 1) for line in Path(f'/proc/{child.pid}/status').read_text().splitlines())
    return bool(int(fields['SigCgt'].strip(), 16) & (1 << (signal.SIGTERM - 1)))


def release_stdout(child, read):
    end = time.monotonic() + 5
    while not caught(child) and time.monotonic() < end:
        if select.select([read], [], [], 0.1)[0]:
            os.read(read, 65536)
    assert caught(child), 'SIGTERM handler never installed'


def run(binary, settled):
    with tempfile.TemporaryDirectory(prefix='blent-signal-review-') as directory:
        root = Path(directory)
        env = configure(root)
        read, write = full_pipe()
        with (root / 'stderr').open('w') as errors:
            child = subprocess.Popen([binary, '--helper', '/bin/true', 'start'], env=env,
                                     stdin=subprocess.DEVNULL, stdout=write, stderr=errors)
        os.close(write)
        try:
            await_routes(child, root)
            assert not caught(child), 'Fixture missed pre-handler startup boundary'
            if settled:
                release_stdout(child, read)
            handler = caught(child)
            child.send_signal(signal.SIGTERM)
            code = child.wait(timeout=15)
            remaining = json.loads((root / 'routes.json').read_text())
            print(json.dumps(dict(settled_control=settled, sigterm_handler_installed=handler,
                                  exit_code=code, remaining_fake_routes=len(remaining))), flush=True)
            assert code == (0 if settled else -signal.SIGTERM)
            assert len(remaining) == (0 if settled else 2)
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()
            os.close(read)


def main():
    assert os.getpid() == 1, 'Must run in isolated PID namespace'
    subprocess.run(['/bin/mount', '-t', 'tmpfs', 'tmpfs', '/dev'], check=True)
    Path('/dev/null').touch()
    binary = str(Path(sys.argv[1]).resolve())
    for settled in [False, True]:
        run(binary, settled)


if __name__ == '__main__':
    main()
