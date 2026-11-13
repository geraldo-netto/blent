#!/usr/bin/python3
"""Review fixture: file-backed Wi-Fi effects, never a native ADB connection."""
import os
from pathlib import Path
import sys
import time


def connect(root, address):
    (root / 'creator').write_text(str(os.getpid()))
    while not (root / 'release').exists():
        time.sleep(0.01)
    (root / 'route').write_text(address)
    (root / 'finished').touch()
    print(f'connected to {address}')


def main():
    root = Path(os.environ['BLENT_REVIEW_ROOT'])
    args = sys.argv[1:]
    with (root / 'calls').open('a') as log:
        log.write(repr(args) + '\n')
    if args == ['devices']:
        print('List of devices attached\nPRIVATEUSB\tdevice')
        if (root / 'route').exists():
            print((root / 'route').read_text() + '\tdevice')
    elif args[0] == 'connect':
        connect(root, args[1])
    elif args[0] == 'disconnect':
        (root / 'route').unlink(missing_ok=True)
    elif args[2:5] == ['shell', 'pm', 'path']:
        print('package:/fake/blent')
    elif args[2:4] == ['tcpip', '5555']:
        print('restarting in TCP mode')
    elif args[2:4] == ['shell', 'ip']:
        print('inet 192.0.2.74/24 scope global wlan0')
    else:
        raise RuntimeError(args)


if __name__ == '__main__':
    main()
