#!/usr/bin/python3
"""T1090 private CLI/server fixture: no real ADB, device or external network."""
import json
import os
from pathlib import Path
import sys
import time

ROOT = Path(__file__).parent


def routes():
    path = ROOT / 'routes.json'
    return json.loads(path.read_text()) if path.exists() else {}


def save(value):
    temporary = ROOT / f'routes.{os.getpid()}'
    temporary.write_text(json.dumps(value))
    temporary.replace(ROOT / 'routes.json')


def wait(name):
    while (ROOT / name).exists():
        time.sleep(0.005)


def create(args):
    remote, local = args[-2:]
    remote = 'tcp:34567' if remote == 'tcp:0' else remote
    current = routes()
    if remote in current:
        return 1
    if (ROOT / 'defer_create').exists():
        (ROOT / 'create_pending').write_text(json.dumps([remote, local]))
        return 1
    current[remote] = local
    save(current)
    (ROOT / 'created').write_text(remote)
    if (ROOT / 'invalid_after_create').exists():
        (ROOT / 'invalid_list').touch()
    wait('hold_create')
    if (ROOT / 'fail_create').exists():
        return 1
    print(remote.removeprefix('tcp:'))
    return 0


def remove(args):
    (ROOT / 'remove_attempt').touch()
    wait('hold_remove')
    if (ROOT / 'fail_remove').exists():
        (ROOT / 'remove_pending').write_text(args[-1])
        return 1
    current = routes()
    current.pop(args[-1], None)
    save(current)
    return 0


def server():
    while True:
        creation = ROOT / 'create_pending'
        if creation.exists() and not (ROOT / 'defer_create').exists():
            remote, local = json.loads(creation.read_text())
            current = routes()
            current[remote] = local
            save(current)
            creation.unlink()
        pending = ROOT / 'remove_pending'
        if pending.exists() and not (ROOT / 'fail_remove').exists():
            current = routes()
            current.pop(pending.read_text(), None)
            save(current)
            pending.unlink()
        time.sleep(0.005)


def listing():
    if (ROOT / 'invalid_list').exists():
        (ROOT / 'list_rejected').touch()
        print('malformed')
        return 0
    for remote, local in routes().items():
        print('Usb', remote, local)
    marker = ROOT / 'hide_after_list'
    if marker.exists():
        marker.unlink()
        (ROOT / 'adb').rename(ROOT / 'adb.saved')
    return 0


def main(args):
    if args == ['--server']:
        return server()
    with (ROOT / 'calls.jsonl').open('a') as stream:
        stream.write(json.dumps(args) + '\n')
    if 'get-serialno' in args:
        print('original-tablet')
        return 0
    if '--list' in args:
        return listing()
    if '--remove' in args:
        return remove(args)
    if 'reverse' in args:
        return create(args)
    if 'shell' in args:
        sys.stdin.read()
        (ROOT / 'shell_entered').touch()
        wait('hold_shell')
        print('Broadcast completed: result=0' if (ROOT / 'reject_shell').exists()
              else 'Broadcast completed: result=1')
        return int((ROOT / 'fail_shell').exists())
    return 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
