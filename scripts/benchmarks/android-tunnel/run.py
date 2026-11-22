#!/usr/bin/env python3
"""Finite hosted TLS/local handoff trials; no installed app or device access."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts/benchmarks'))
import profile_process

TASKS = {'tls-write': 'tunnelHandoffBenchmark', 'local-write': 'localHandoffBenchmark',
         'tls-read': 'tunnelReadBenchmark', 'mux-allocation': 'connectionAllocationBenchmark'}
SCOPES = {
    'tls-write': 'Hosted Linux JDK loopback TLS 1.3; production framing/native I/O; production gather limits 0/16384; not Android native or full reactor',
    'local-write': 'Hosted Linux JDK plaintext loopback TCP; production local consumer versus direct native transaction; not Android LocalSocket or full reactor',
    'tls-read': 'Hosted Linux JDK TLS receive; production exact framing versus benchmark 16-KiB read-ahead feeding the same reader; not Android native or full reactor',
    'mux-allocation': 'Hosted JDK current production mux idle/data scheduling allocation and CPU; no native I/O or Android ART acceptance',
}


def domain(kind):
    if kind == 'mux-allocation':
        return dict(warmup_trials=4, measured_trials=10, iterations_per_trial=200000, modes=['idle', 'data'])
    return dict(warmup_trials=16, measured_trials=48, frames_per_trial=60,
                payload_bytes=[64, 4096, 16384, 65536], rates=[0, 60])


def inputs():
    paths = list((ROOT / 'android/app/src/main/java').rglob('*.kt'))
    for folder in ('android-tunnel', 'android-hosted'):
        paths += [p for p in (ROOT / 'scripts/benchmarks' / folder).rglob('*')
                  if p.suffix in ('.kt', '.py', '.gradle')]
    paths += list((ROOT / 'testdata/connection-tls').glob('*'))
    paths += [ROOT / p for p in ('android/app/build.gradle.kts', 'android/build.gradle.kts',
              'android/settings.gradle.kts', 'android/gradle.properties',
              'android/gradle/wrapper/gradle-wrapper.properties', 'scripts/benchmarks/profile_process.py')]
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--kind', choices=tuple(TASKS), default='tls-write')
    parser.add_argument('--peer-gather-bytes', type=int, choices=(0, 16384), default=0)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True)
    before = inputs()
    metadata = dict(source_sha256=before,
        parent_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        platform=platform.platform(), java=subprocess.run(['java', '-version'], capture_output=True, text=True, check=True).stderr,
        domain=domain(args.kind),
        kind=args.kind, peer_gather_bytes=args.peer_gather_bytes, scope=SCOPES[args.kind])
    (output / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
    command = [str(ROOT / 'android/gradlew'), '-p', str(ROOT / 'android'), '--offline', '--no-daemon',
               '-I', str(ROOT / 'scripts/benchmarks/android-tunnel/benchmark.init.gradle'),
               ':app:' + TASKS[args.kind], '-PblentTunnelOutput=' + str(output / 'results.json'),
               '-PblentPeerGatherBytes=' + str(args.peer_gather_bytes)]
    status = profile_process.logged(command, dict(os.environ), output / 'run.log', ROOT)
    if status != 0:
        raise RuntimeError(f'Benchmark failed ({status}); retained {output}/run.log')
    if inputs() != before:
        raise RuntimeError('Benchmark source inputs changed')
    results = json.loads((output / 'results.json').read_text())
    expected = metadata['domain']['warmup_trials'] + metadata['domain']['measured_trials']
    if len(results) != expected:
        raise RuntimeError(f'Incomplete {expected}-trial domain')
    (output / 'complete.json').write_text(json.dumps(dict(trials=len(results), unchanged_inputs=True)) + '\n')
    print(output / 'results.json')


if __name__ == '__main__':
    main()
