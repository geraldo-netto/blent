#!/usr/bin/python3
"""Review actual CLI only inside a private PID/user/mount/network namespace."""
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time


def await_file(path):
    until = time.monotonic() + 15
    while not path.exists():
        assert time.monotonic() < until, str(path)
        time.sleep(0.01)


def environment(root):
    shutil.copyfile(Path(__file__).with_name('fake_adb.py'), root / 'adb')
    (root / 'adb').chmod(0o700)
    return dict(os.environ, HOME=str(root), XDG_CONFIG_HOME=str(root / 'config'),
                XDG_RUNTIME_DIR=str(root), XDG_STATE_HOME='private-no-broker',
                PATH=str(root), RUST_LOG='off', BLENT_REVIEW_ROOT=str(root),
                LLVM_PROFILE_FILE=str(root / 'cli-%p.profraw'))


def observe(root, child, number):
    await_file(root / 'creator')
    creator = int((root / 'creator').read_text())
    fields = dict(line.split(':', 1) for line in Path(f'/proc/{child.pid}/status').read_text().splitlines())
    mask = int(fields['SigCgt'].strip(), 16)
    if number:
        child.send_signal(number)
        result = child.wait(timeout=5)
        assert result == -number, result
        assert Path(f'/proc/{creator}').exists()
    (root / 'release').touch()
    await_file(root / 'finished')
    if number:
        os.waitpid(creator, 0)
    result = child.wait(timeout=5)
    config = root / 'config/blent/config.toml'
    saved = config.exists() and '192.0.2.74:5555' in config.read_text()
    output = dict(signal=number, exit=result, caught=bool(mask & (1 << (number - 1))) if number else None,
                  late_route=(root / 'route').exists(), saved=saved,
                  disconnect="['disconnect'," in (root / 'calls').read_text())
    print(json.dumps(output), flush=True)
    assert saved == (number == 0), output
    assert output['late_route'] and not output['disconnect'], output


def run(binary, number):
    with tempfile.TemporaryDirectory(prefix='blent-wifi-cli-') as directory:
        root = Path(directory)
        with (root / 'output').open('w') as output:
            child = subprocess.Popen([binary, 'wifi'], env=environment(root),
                                     stdin=subprocess.DEVNULL, stdout=output, stderr=output)
            try:
                observe(root, child, number)
            finally:
                (root / 'release').touch()
                if child.poll() is None:
                    child.kill()
                child.wait()


if __name__ == '__main__':
    for selected in [0, signal.SIGINT, signal.SIGTERM]:
        run(sys.argv[1], selected)
