#!/usr/bin/env python3
"""T1755: finite hosted TLS handoff trials; no installed app or device access."""
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
    output = parser.parse_args().output.resolve()
    output.mkdir(parents=True)
    before = inputs()
    metadata = dict(source_sha256=before,
        parent_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        platform=platform.platform(), java=subprocess.run(['java', '-version'], capture_output=True, text=True, check=True).stderr,
        domain=dict(warmup_trials=16, measured_trials=48, frames_per_trial=60,
                    payload_bytes=[64, 4096, 16384, 65536], rates=[0, 60]),
        scope='Hosted Linux JDK loopback TLS 1.3; production framing/native I/O; production gather limits 0/16384; not Android native or full reactor')
    (output / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
    command = [str(ROOT / 'android/gradlew'), '-p', str(ROOT / 'android'), '--offline', '--no-daemon',
               '-I', str(ROOT / 'scripts/benchmarks/android-tunnel/benchmark.init.gradle'),
               ':app:tunnelHandoffBenchmark', '-PblentTunnelOutput=' + str(output / 'results.json')]
    status = profile_process.logged(command, dict(os.environ), output / 'run.log', ROOT)
    if status != 0:
        raise RuntimeError(f'Benchmark failed ({status}); retained {output}/run.log')
    if inputs() != before:
        raise RuntimeError('Benchmark source inputs changed')
    results = json.loads((output / 'results.json').read_text())
    if len(results) != 64:
        raise RuntimeError('Incomplete 64-trial domain')
    (output / 'complete.json').write_text(json.dumps(dict(trials=len(results), unchanged_inputs=True)) + '\n')
    print(output / 'results.json')


if __name__ == '__main__':
    main()
