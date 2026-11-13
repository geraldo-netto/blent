#!/usr/bin/env python3
"""T1258: real ADB effects with controlled host acknowledgement gates.

Each invocation waits for its actual ADB child. Gates retain only this wrapper,
never a detached ADB mutation. The probe owns gate release on success/failure.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def record(directory, event, **fields):
    with (directory / 'commands.jsonl').open('a') as stream:
        stream.write(json.dumps(dict(event=event, monotonic_ns=time.monotonic_ns(), **fields)) + '\n')


def gate(directory, marker):
    (directory / marker).write_text('original command retained\n')
    while not (directory / 'release').exists():
        time.sleep(0.01)


def execute(directory, args):
    result = subprocess.run(['/usr/bin/adb', *args], capture_output=True)
    record(directory, 'real_adb_joined', args=args, status=result.returncode,
           stdout=result.stdout.decode(errors='replace'), stderr=result.stderr.decode(errors='replace'))
    return result


def main():
    directory = Path(os.environ['T1258_RUN'])
    mode = os.environ['T1258_MODE']
    args = sys.argv[1:]
    creating = '--no-rebind' in args
    removing = '--remove' in args
    if removing and mode == 'reconnect':
        gate(directory, 'remove_waiting')
    result = execute(directory, args)
    if creating and result.returncode == 0:
        (directory / 'created').write_text(json.dumps(args))
        if mode == 'cancel':
            gate(directory, 'create_waiting')
        if mode == 'lost_ack':
            record(directory, 'controlled_local_acknowledgement_loss')
            return 1
    if removing and mode == 'remove_delay':
        gate(directory, 'remove_waiting')
    sys.stdout.buffer.write(result.stdout)
    sys.stderr.buffer.write(result.stderr)
    return result.returncode


if __name__ == '__main__':
    sys.exit(main())
