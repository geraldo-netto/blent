#!/usr/bin/python3
"""T1695: two real daemon starts; only the admitted lifetime owns startup."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

from signals import configure, await_condition, routes, reap, open_writer
from drain import pending_removals


def start(binary, root, env, label):
    log = root / label
    with log.open('w') as output:
        child = subprocess.Popen([binary, '--helper', '/bin/true', 'start'], env=env,
                                 stdout=output, stderr=output)
    return child, log


def await_routes(child, root, log, description):
    try:
        await_condition(child, lambda: len(routes(root)) == 2, description)
    except AssertionError as error:
        raise AssertionError(f'{error}\n{log.read_text()}') from error


def rejected(binary, root, env, owner):
    child, log = start(binary, root, env, 'rejected')
    try:
        assert child.wait(timeout=15) == 1, log.read_text()
        assert 'already running' in log.read_text(), log.read_text()
        assert (root / '.local/share/blent/blent.pid').read_text() == str(owner)
    finally:
        reap(child)


def stop_drain(binary, root, env, owner, log):
    (root / 'hold-removal').touch()
    owner.send_signal(signal.SIGTERM)
    await_condition(owner, lambda: 'Waiting for native retirement' in log.read_text(), 'entry drain')
    assert pending_removals(root) == 1, 'T1698 original removal already exited'
    rejected(binary, root, env, owner.pid)
    (root / 'hold-removal').unlink()
    try:
        assert owner.wait(timeout=15) == 0, log.read_text()
    except subprocess.TimeoutExpired as error:
        raise AssertionError(f'{log.read_text()}\nRoutes: {routes(root)}\n'
                             f'Calls: {(root / "adb-calls.txt").read_text()}') from error
    assert not (root / '.local/share/blent/blent.pid').exists()
    assert not routes(root)


def reuse(binary, root, env):
    replacement, log = start(binary, root, env, 'replacement')
    try:
        await_routes(replacement, root, log, 'replacement routes')
        replacement.send_signal(signal.SIGTERM)
        assert replacement.wait(timeout=15) == 0, log.read_text()
    finally:
        reap(replacement)


def contenders(binary, root, env, config):
    content = config.read_bytes()
    ready = root / 'ready-config/blent/config.toml'
    ready.parent.mkdir(parents=True)
    ready.write_bytes(content)
    config.unlink()
    os.mkfifo(config)
    slow, slow_log = start(binary, root, env, 'slow')
    writer = open_writer(slow, config)
    fast_env = dict(env, XDG_CONFIG_HOME=str(ready.parent.parent))
    fast, fast_log = start(binary, root, fast_env, 'fast')
    try:
        # Slow is a real same-user `blent start`, but owns no startup lease yet.
        await_routes(fast, root, fast_log, 'one winner beside pre-admission contender')
        os.write(writer, content)
        os.close(writer)
        writer = None
        assert slow.wait(timeout=15) == 1, slow_log.read_text()
        assert 'already running' in slow_log.read_text(), slow_log.read_text()
        rejected(binary, root, fast_env, fast.pid)
        stop_drain(binary, root, fast_env, fast, fast_log)
        reuse(binary, root, fast_env)
    finally:
        reap(slow)
        if writer is not None:
            os.close(writer)
        (root / 'hold-removal').unlink(missing_ok=True)
        reap(fast)


def main():
    assert os.getpid() == 1, 'Must run in isolated PID namespace'
    subprocess.run(['/bin/mount', '-t', 'tmpfs', 'tmpfs', '/dev'], check=True)
    Path('/dev/null').touch()
    with tempfile.TemporaryDirectory(prefix='blent-t1695-') as directory:
        root = Path(directory)
        env, config = configure(root)
        env['RUST_LOG'] = 'warn'
        env['BLENT_SESSION_STOP_MS'] = '100'
        env['BLENT_REAP_TIMEOUT_MS'] = '100'
        contenders(str(Path(sys.argv[1]).resolve()), root, env, config)
        print(json.dumps(dict(case='T1695', pre_admission_contender=True,
                             exclusive_start=True, retained_drain=True, settled_reuse=True)))


if __name__ == '__main__':
    main()
