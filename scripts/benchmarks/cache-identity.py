#!/usr/bin/env python3
"""T1493: paired private release-test cache preparation observations.

Runs only the named opt-in timing probe. Each child owns temporary configuration,
one synthetic attachment and ten bounded numeric observation rows. No tablet or
capture session is started. Every original child is waited on before the next.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import statistics
import subprocess

PROBE = 'selection::worker::cache::tests::t1493_measure_cache_open_and_cached_selection'


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--helper', type=Path, required=True)
    parser.add_argument('--packaged-bin', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repeats', type=int, choices=range(1, 6), default=3)
    return parser.parse_args()


def identity(path):
    metadata = path.stat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 512 * 1024 * 1024:
        raise ValueError('benchmark input must be a regular file of at most 512 MiB')
    digest = hashlib.sha256()
    with path.open('rb') as source:
        total = 0
        for block in iter(lambda: source.read(65536), b''):
            total += len(block)
            if total > 512 * 1024 * 1024:
                raise ValueError('benchmark input grew beyond 512 MiB')
            digest.update(block)
    after = path.stat()
    if (metadata.st_ino, metadata.st_size, metadata.st_mtime_ns, metadata.st_ctime_ns) != (after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns):
        raise ValueError('benchmark input changed during identity collection')
    return {'path': str(path), 'bytes': total, 'sha256': digest.hexdigest()}


def observe(binary, helper, path):
    env = dict(os.environ, PATH=path, BLENT_T1493_HELPER=str(helper),
               BLENT_RUNTIME_HASH_CACHE_ENTRIES='64')
    result = subprocess.run([str(binary), '--exact', PROBE, '--ignored', '--nocapture'],
                            env=env, text=True, capture_output=True, check=True)
    match = re.search(r'T1493_MEASURE (\{[^\n]+\})', result.stdout)
    if not match:
        raise ValueError('timing probe did not publish its observations')
    row = json.loads(match.group(1))
    if len(row['observations']) != 10 or len(row['components']) != 10:
        raise ValueError('unexpected probe observation count')
    return row


def summarize(rows):
    groups = {}
    for runtime in ('system', 'packaged'):
        groups[runtime] = {}
        for variant in ('baseline', 'candidate'):
            selected = [row for row in rows if (row['runtime'], row['variant']) == (runtime, variant)]
            repeated = [sample for row in selected for sample in row['observations'][1:]]
            groups[runtime][variant] = {
                'repeat_samples': len(repeated),
                'open_median_us': statistics.median(sample['open_us'] for sample in repeated),
                'load_median_us': statistics.median(sample['load_us'] for sample in repeated),
                'first_open_us': [row['observations'][0]['open_us'] for row in selected],
            }
    return groups


def run(args):
    binaries = {'baseline': args.baseline.resolve(), 'candidate': args.candidate.resolve()}
    result = {'binaries': {name: identity(path) for name, path in binaries.items()},
              'helper': identity(args.helper.resolve()), 'trials': []}
    paths = {'system': '/usr/bin:/bin',
             'packaged': str(args.packaged_bin.resolve()) + ':/usr/bin:/bin'}
    for repeat in range(args.repeats):
        variants = ('candidate', 'baseline') if repeat % 2 else ('baseline', 'candidate')
        for runtime, path in paths.items():
            for variant in variants:
                row = observe(binaries[variant], args.helper.resolve(), path)
                row.update(runtime=runtime, variant=variant, repeat=repeat)
                result['trials'].append(row)
                args.output.write_text(json.dumps(result, indent=2) + '\n')
                print(f'T1493: {len(result["trials"])}/{args.repeats * 4} private probe processes completed', flush=True)
    result['summary'] = summarize(result['trials'])
    args.output.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    run(arguments())
