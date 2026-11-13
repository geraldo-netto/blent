#!/usr/bin/python3
"""Permanent private file-backed ADB fixture; never invokes a real ADB server."""
import json
import os
from pathlib import Path
import sys


def reverse(root, args):
    path = root / 'routes.json'
    routes = json.loads(path.read_text()) if path.exists() else {}
    action = args[3]
    if action == '--list':
        print(''.join(f'UsbFfs {remote} {local}\n' for remote, local in routes.items()), end='')
    elif action == '--no-rebind':
        routes[args[4]] = args[5]
        path.write_text(json.dumps(routes))
    elif action == '--remove':
        routes.pop(args[4], None)
        path.write_text(json.dumps(routes))
    else:
        raise RuntimeError(args)


def main():
    root = Path(os.environ['BLENT_REVIEW_ROOT'])
    args = sys.argv[1:]
    with (root / 'adb-calls.txt').open('a') as calls:
        calls.write(repr(args) + '\n')
    if args == ['devices']:
        print('List of devices attached\nRECHECKUSB\tdevice')
    elif args == ['version']:
        print('Android Debug Bridge version 1.0.41')
    elif args[2:4] == ['shell', 'getprop']:
        print('RECHECKPHYSICAL')
    elif args[2:5] == ['shell', 'pm', 'path']:
        print('package:/fake/blent')
    elif args[2] == 'reverse':
        reverse(root, args)
    elif args[2] == 'shell':
        sys.stdin.read()
    else:
        raise RuntimeError(args)


if __name__ == '__main__':
    main()
