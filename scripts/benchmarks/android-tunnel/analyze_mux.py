#!/usr/bin/env python3
"""Compare separate current-source mux runs; ratios are not paired causal proof."""
import argparse
import json
from pathlib import Path
import statistics


def summarize(path):
    rows = json.loads(path.read_text())
    if len(rows) != 14 or sum(row['warmup'] for row in rows) != 4:
        raise ValueError('Incomplete mux allocation domain')
    return {mode: summarize_mode(rows, mode) for mode in ('idle', 'data')}


def summarize_mode(rows, mode):
    selected = [row for row in rows if not row['warmup'] and row['mode'] == mode]
    if sorted(row['trial'] for row in selected) != [2, 3, 4, 5, 6]:
        raise ValueError('Incomplete measured trial domain')
    for row in selected:
        if row['iterations'] != 200000:
            raise ValueError('Unexpected iteration domain')
    return {key + '_per_pass': statistics.median(row[key] / row['iterations'] for row in selected)
            for key in ('driver_allocated_bytes', 'driver_cpu_ns', 'elapsed_ns')}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    args = parser.parse_args()
    before, after = summarize(args.before), summarize(args.after)
    comparisons = {}
    for mode in ('idle', 'data'):
        comparisons[mode] = dict(before=before[mode], after=after[mode],
            cpu_ratio=after[mode]['driver_cpu_ns_per_pass'] / before[mode]['driver_cpu_ns_per_pass'],
            wall_ratio=after[mode]['elapsed_ns_per_pass'] / before[mode]['elapsed_ns_per_pass'],
            saved_bytes_per_pass=before[mode]['driver_allocated_bytes_per_pass'] - after[mode]['driver_allocated_bytes_per_pass'])
    print(json.dumps(dict(scope='separate hosted JVM runs, five measured trials per mode/run', comparisons=comparisons), indent=2))


if __name__ == '__main__':
    main()
