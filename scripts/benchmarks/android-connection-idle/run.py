#!/usr/bin/env python3
"""T918: offline hosted actor CPU/allocation and notification latency measurements."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
DIRECTORY = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / 'scripts/tests'))
from test_secure_connection_interop import SecureConnectionInteropTest


def inputs():
    paths = list((ROOT / 'android/app/src').rglob('*.kt'))
    paths += list(DIRECTORY.rglob('*.kt'))
    paths += list((ROOT / 'scripts/benchmarks/android-hosted').rglob('*.kt'))
    paths += [Path(__file__).resolve(), DIRECTORY / 'benchmark.init.gradle']
    paths += [ROOT / name for name in [
        'android/app/build.gradle.kts', 'android/build.gradle.kts', 'android/settings.gradle.kts',
        'android/gradle.properties', 'scripts/coverage/android.init.gradle', 'scripts/tests/test_secure_connection_interop.py',
    ]]
    return {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(paths)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--allocations', action='store_true')
    modes.add_argument('--active', action='store_true')
    arguments = parser.parse_args()
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    original = inputs()
    (output / 'sources.json').write_text(json.dumps(original, indent=2) + '\n')
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    (output / 'revision.txt').write_text(head + '\n')
    entry = 'DeadlineAllocationBenchmarkKt' if arguments.allocations else 'ConnectionIdleBenchmarkKt'
    if arguments.active:
        entry = 'ActiveAllocationBenchmarkKt'
    SecureConnectionInteropTest().command([
        str(ROOT / 'android/gradlew'), '-p', str(ROOT / 'android'), '--offline', '--no-daemon', '--max-workers=2',
        '-I', str(ROOT / 'scripts/coverage/android.init.gradle'), '-I', str(DIRECTORY / 'benchmark.init.gradle'),
        '-PblentIdleMain=com.blent.connection.' + entry,
        '-PblentIdleOutput=' + str(output / 'results.json'), ':app:connectionIdleBenchmark',
    ], dict(os.environ), output / 'run.log')
    if inputs() != original:
        raise RuntimeError('T918 measurement inputs changed during the run')
    if arguments.active:
        report = json.loads((output / 'results.json').read_text())
        if len(report['trials']) != 12:
            raise RuntimeError('T1768 incomplete active trial domain')
    print(output / 'results.json')


if __name__ == '__main__':
    main()
