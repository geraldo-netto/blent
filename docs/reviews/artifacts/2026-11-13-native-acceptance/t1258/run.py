#!/usr/bin/env python3
"""T1258 focused trial. Owns exactly one unrelated control mapping; never remove-all.

The Rust probe owns production mappings and waits for native retirement. Local
control sockets stay reserved until exact control cleanup. No media invitations,
Android settings changes or package changes are performed.
"""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
SERIAL = '8002RH1010011900'
BINARY = '/tmp/blent-t1258/target/debug/blent-t1258-native'


def adb(*args):
    result = subprocess.run(['/usr/bin/adb', '-s', SERIAL, *args], capture_output=True, text=True)
    with (ROOT / 'controller.jsonl').open('a') as stream:
        stream.write(json.dumps(dict(args=args, status=result.returncode, stdout=result.stdout,
                                    stderr=result.stderr, monotonic_ns=time.monotonic_ns())) + '\n')
    return result


def listing():
    result = adb('reverse', '--list')
    if result.returncode:
        raise RuntimeError(result.stderr)
    return {tuple(line.split()[1:]) for line in result.stdout.splitlines() if line.strip()}


def reserve():
    listener = socket.socket()
    listener.bind(('127.0.0.1', 0))
    return listener


def create_control(remote, local):
    result = adb('reverse', '--no-rebind', remote, local)
    if result.returncode:
        raise RuntimeError('Control admission rejected: ' + result.stderr)


def remove_control(remote, local):
    if (remote, local) not in listing():
        raise RuntimeError('Original control mapping no longer present; not deleting another owner')
    result = adb('reverse', '--remove', remote)
    if result.returncode:
        raise RuntimeError('Control cleanup not acknowledged: ' + result.stderr)


def run_case(mode, name=None):
    directory = ROOT / (name or mode)
    directory.mkdir()
    before = listing()
    environment = dict(os.environ, T1258_MODE=mode, T1258_RUN=str(directory),
                       T1258_SERIAL=SERIAL, T1258_WRAPPER=str(ROOT / 'adb-wrapper.py'))
    with (directory / 'probe.log').open('w') as output:
        result = subprocess.run([BINARY], env=environment, stdout=output, stderr=subprocess.STDOUT)
    immediate = listing()
    after = restored_routes(before) if mode == 'reconnect' else immediate
    report = dict(case=mode, immediate_after=sorted(immediate), status=result.returncode, before=sorted(before), after=sorted(after),
                  original_mappings_preserved=before.issubset(immediate), baseline_restored=before.issubset(after),
                  temporary_mappings_removed=after.issubset(before))
    (directory / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report), flush=True)
    if result.returncode or after != before:
        raise RuntimeError('Native case did not settle with original mapping set: ' + mode)


def control_cases():
    baseline = listing()
    with reserve() as remote_socket, reserve() as local_socket:
        remote = 'tcp:' + str(remote_socket.getsockname()[1])
        local = 'tcp:' + str(local_socket.getsockname()[1])
        create_control(remote, local)
        try:
            denied = adb('reverse', '--no-rebind', remote, 'tcp:1')
            preserved = (remote, local) in listing()
            report = dict(case='no_rebind', refused=denied.returncode != 0, original_preserved=preserved)
            (ROOT / 'no-rebind.json').write_text(json.dumps(report, indent=2) + '\n')
            if not report['refused'] or not preserved:
                raise RuntimeError('ADB no-rebind did not preserve the original endpoint')
            for mode in ('normal', 'drop', 'cancel', 'lost_ack', 'remove_delay'):
                run_case(mode)
        finally:
            remove_control(remote, local)
    if listing() != baseline:
        raise RuntimeError('Control cleanup changed baseline mappings')


def restored_routes(expected):
    observations = 0
    while True:
        current = listing()
        if current == expected:
            return current
        if current - expected:
            raise RuntimeError('Unexpected mapping after reconnect; preserving it')
        observations += 1
        if observations % 50 == 0:
            print(json.dumps(dict(event='waiting_for_display_route_repair', present=sorted(current))), flush=True)
        time.sleep(0.1)


def main():
    if sys.argv[1:] == ['reconnect']:
        before = subprocess.check_output(['/usr/bin/adb', 'devices', '-l'], text=True)
        run_case('reconnect', 'reconnect-confirmed')
        after = subprocess.check_output(['/usr/bin/adb', 'devices', '-l'], text=True)
        (ROOT / 'transport-identity.json').write_text(json.dumps(dict(before=before, after=after), indent=2) + '\n')
    else:
        control_cases()


if __name__ == '__main__':
    main()
