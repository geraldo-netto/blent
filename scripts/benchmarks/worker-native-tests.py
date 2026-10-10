#!/usr/bin/env python3
"""Permanent T809 disposable-device acceptance. No downloads or production app changes."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

PACKAGE = 'com.blent.workertest'
RECEIVER = PACKAGE + '/com.blent.WorkerAcceptanceReceiver'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adb', type=Path, required=True)
    parser.add_argument('--serial', required=True)
    parser.add_argument('--project', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    provenance = args.project / 'worker-provenance.json'
    metadata = json.loads(provenance.read_text())
    apk = args.project / 'android/app/build/outputs/apk/debug/app-debug.apk'
    assert metadata['package'] == PACKAGE
    assert hashlib.sha256(apk.read_bytes()).hexdigest() == metadata['apk_sha256']
    args.output.mkdir(parents=True, exist_ok=False)
    adb = [str(args.adb), '-s', args.serial]

    def command(*values):
        return subprocess.check_output(adb + list(values), text=True, stderr=subprocess.DEVNULL, timeout=30)

    def read(name):
        return json.loads(command('shell', 'run-as', PACKAGE, 'cat', 'files/' + name))

    def await_file(name, run_id=None):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            try:
                result = read(name)
                if run_id is None or result.get('run_id') == run_id:
                    return result
            except (subprocess.CalledProcessError, json.JSONDecodeError):
                pass
            time.sleep(.2)
        raise AssertionError('Native worker acceptance deadline: ' + name)

    def live_processes():
        return [line for line in command('shell', 'ps', '-A').splitlines() if line.split()[-1].startswith(PACKAGE)]

    command('install', '-r', str(apk))
    try:
        for trial in range(2):
            run_id = 'production-' + str(trial)
            command('shell', 'am', 'force-stop', PACKAGE)
            command('shell', 'am', 'broadcast', '-n', RECEIVER, '--es', 'run_id', run_id)
            result = await_file('workers.json', run_id)
            assert 'error' not in result, result
            assert len({row['pid'] for row in result['workers']}) == 4
            assert result['cleanup_hang_contained'] and result['worker_death_observed']
            assert result['held_owner_stop_ns'] < 4_000_000_000
            assert result['sibling_heartbeats'] > 0
            assert result['native_metadata']['width'] == 640
            (args.output / (run_id + '.json')).write_text(json.dumps(result, indent=2) + '\n')
            # Only the controller may remain. Each idle resource process must exit.
            assert all(line.split()[-1] == PACKAGE for line in live_processes()), live_processes()
        command('shell', 'run-as', PACKAGE, 'rm', '-f', 'files/owner-death.json')
        command('shell', 'am', 'broadcast', '-n', RECEIVER, '--ez', 'owner_death', 'true')
        result = await_file('owner-death.json')
        deadline = time.monotonic() + 5
        while live_processes() and time.monotonic() < deadline:
            time.sleep(.2)
        assert not live_processes(), 'Workers survived their owner'
        result['all_processes_exited'] = True
        (args.output / 'owner-death.json').write_text(json.dumps(result, indent=2) + '\n')
        shutil.copy2(provenance, args.output / 'provenance.json')
        print('T809 native worker death, hang, reconnect, handle, metadata and parent-loss tests passed')
    finally:
        command('shell', 'am', 'force-stop', PACKAGE)
        command('uninstall', PACKAGE)


if __name__ == '__main__':
    main()
