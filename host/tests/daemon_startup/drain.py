#!/usr/bin/python3
"""T1509: actual daemon entry; fake routes retain cleanup past the body deadline."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

from signals import configure, await_condition, routes, reap


def retained_drain(binary):
    with tempfile.TemporaryDirectory(prefix='blent-t1509-') as directory:
        root = Path(directory)
        env, _ = configure(root)
        env['RUST_LOG'] = 'warn'
        (root / 'hold-removal').touch()
        errors = root / 'stderr'
        with errors.open('w') as output:
            child = subprocess.Popen([binary, '--helper', '/bin/true', 'start'], env=env,
                                     stdout=output, stderr=output)
        try:
            await_condition(child, lambda: len(routes(root)) == 2, 'original routes')
            child.send_signal(signal.SIGTERM)
            await_condition(child, lambda: 'Waiting for native retirement' in errors.read_text(), 'entry drain diagnostic')
            pid_file = root / '.local/share/blent/blent.pid'
            assert pid_file.read_text() == str(child.pid), 'T1509 PID disappeared during drain'
            child.send_signal(signal.SIGINT)
            await_condition(child, lambda: 'Termination requested again' in errors.read_text(), 'repeated signal diagnostic')
            (root / 'hold-removal').unlink()
            assert child.wait(timeout=15) == 0, errors.read_text()
            assert not pid_file.exists(), 'T1509 settled exit retained PID'
            assert not routes(root), 'T1509 settled exit retained routes'
            print(json.dumps(dict(case='T1509', retained_pid=True, repeated_signal=True, settled=True)))
        finally:
            (root / 'hold-removal').unlink(missing_ok=True)
            reap(child)


def cli_dispatch(binary):
    with tempfile.TemporaryDirectory(prefix='blent-t1509-cli-') as directory:
        root = Path(directory)
        env, _ = configure(root)
        cases = [(['stop'], 0), (['status'], 0), (['list-displays'], 0),
                 (['cameras', '--width', '0'], 1), (['audio', '--buffer-ms', '0'], 1)]
        for args, expected in cases:
            result = subprocess.run([binary, *args], env=env, capture_output=True, text=True)
            assert result.returncode == expected, (args, result.stdout, result.stderr)
        assert not (root / 'adb-calls.txt').exists(), 'T1509 passive/invalid command touched ADB'
        assert not (root / '.local/share/blent/blent.pid').exists()
        print(json.dumps(dict(case='T1509 CLI dispatch', programs=len(cases), passed=len(cases))))


def main():
    assert os.getpid() == 1, 'Must run in isolated PID namespace'
    subprocess.run(['/bin/mount', '-t', 'tmpfs', 'tmpfs', '/dev'], check=True)
    Path('/dev/null').touch()
    binary = str(Path(sys.argv[1]).resolve())
    retained_drain(binary)
    cli_dispatch(binary)


if __name__ == '__main__':
    main()
