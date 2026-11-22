#!/usr/bin/env python3
"""Compare two finite hosted actor runs; separate-run medians are point estimates."""
import argparse
import json
import math
from pathlib import Path
import statistics


def p95(row):
    values = sorted(row['latency_ns'])
    if len(values) != row['events']:
        raise ValueError('Incomplete event latency domain')
    return values[math.ceil(len(values) * .95) - 1]


def summarize(path, fps):
    rows = json.loads(path.read_text())['trials']
    if len(rows) != 12 or sum(row['warmup'] for row in rows) != 2:
        raise ValueError('Incomplete active actor trial domain')
    selected = [row for row in rows if not row['warmup'] and row['fps'] == fps]
    if sorted(row['trial'] for row in selected) != [1, 2, 3, 4, 5]:
        raise ValueError('Incomplete measured actor domain')
    expected = 10000 if fps == 0 else 120
    if any(row['events'] != expected for row in selected):
        raise ValueError('Unexpected event count')
    result = {key + '_per_event': statistics.median(row[key] / row['events'] for row in selected)
              for key in ('actor_cpu_ns', 'actor_allocated_bytes', 'driver_cpu_ns', 'driver_allocated_bytes')}
    result['total_allocated_bytes_per_event'] = statistics.median(
        (row['actor_allocated_bytes'] + row['driver_allocated_bytes']) / row['events'] for row in selected)
    result['total_cpu_ns_per_event'] = statistics.median(
        (row['actor_cpu_ns'] + row['driver_cpu_ns']) / row['events'] for row in selected)
    result['p95_latency_ns'] = statistics.median(p95(row) for row in selected)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    args = parser.parse_args()
    comparisons = []
    for fps in (0, 60):
        before, after = summarize(args.before, fps), summarize(args.after, fps)
        comparisons.append(dict(fps=fps, before=before, after=after,
            total_cpu_ratio=after['total_cpu_ns_per_event'] / before['total_cpu_ns_per_event'],
            latency_p95_ratio=after['p95_latency_ns'] / before['p95_latency_ns'],
            saved_total_bytes_per_event=before['total_allocated_bytes_per_event'] - after['total_allocated_bytes_per_event']))
    print(json.dumps(dict(scope='separate hosted runs; five measured trials per rate/run', comparisons=comparisons), indent=2))


if __name__ == '__main__':
    main()
