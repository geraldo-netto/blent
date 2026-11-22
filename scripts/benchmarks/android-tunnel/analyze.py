#!/usr/bin/env python3
"""Summarize the three paired measured trials; retain all raw latency samples."""
import argparse
import itertools
import json
import math
from pathlib import Path
import statistics


def p95(row):
    samples = sorted(row['latency_ns'])
    if len(samples) != row['frames']:
        raise ValueError('Incomplete latency domain')
    return samples[math.ceil(len(samples) * 0.95) - 1]


def mode_summary(rows):
    return dict(trials=len(rows),
        cpu_ns_median=statistics.median(r['driver_and_io_cpu_ns'] for r in rows),
        p95_ns_median=statistics.median(p95(r) for r in rows),
        tls_records_per_frame=sorted({r['tls_application_records'] / r['frames'] for r in rows}),
        native_completions_per_frame=sorted({r['native_completions_including_ack'] / r['frames'] for r in rows}),
        driver_poll_calls=[r['driver_poll_calls'] for r in rows],
        driver_wait_calls=[r['driver_wait_calls'] for r in rows],
        observed_ready_hints=[r['ready_hints_observed'] for r in rows])


def comparison(rows, fps, size):
    selected = [r for r in rows if r['fps'] == fps and r['payload_bytes'] == size]
    baseline = sorted([r for r in selected if not r['combined']], key=lambda r: r['trial'])
    combined = sorted([r for r in selected if r['combined']], key=lambda r: r['trial'])
    if [r['trial'] for r in baseline] != [1, 2, 3] or [r['trial'] for r in combined] != [1, 2, 3]:
        raise ValueError('Incomplete paired trial domain')
    pairs = list(zip(baseline, combined))
    return dict(fps=fps, payload_bytes=size, baseline=mode_summary(baseline), combined=mode_summary(combined),
        paired_cpu_ratio_median=statistics.median(b['driver_and_io_cpu_ns'] / a['driver_and_io_cpu_ns'] for a, b in pairs),
        paired_p95_ratio_median=statistics.median(p95(b) / p95(a) for a, b in pairs))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('results', type=Path)
    args = parser.parse_args()
    rows = json.loads(args.results.read_text())
    if len(rows) != 64 or sum(row['warmup'] for row in rows) != 16:
        raise ValueError('Incomplete warmup/measurement domain')
    measured = [r for r in rows if not r['warmup']]
    summary = [comparison(measured, fps, size) for fps, size in itertools.product((0, 60), (64, 4096, 16384, 65536))]
    print(json.dumps(dict(warmup_trials=16, measured_trials=48, comparisons=summary), indent=2))


if __name__ == '__main__':
    main()
