#!/usr/bin/env python3
"""T768/T769/T773/T778: offline, single-session C microbenchmarks; no live devices."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shlex
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'scripts/benchmarks'


def run(command, environment=None):
    return subprocess.check_output(command, cwd=ROOT, text=True, stderr=subprocess.PIPE, env=environment)


def build(directory):
    # This comparison intentionally includes ISA-specific binaries. Fail before
    # executing them on a host without the required CPU/OS capabilities.
    flags = set(next(line.split(':', 1)[1] for line in Path('/proc/cpuinfo').read_text().splitlines()
                     if line.startswith('flags')).split())
    if not {'avx2', 'avx512f', 'avx512bw', 'avx512vl'} <= flags:
        raise RuntimeError('This local SIMD comparison requires AVX2 and AVX-512 F/BW/VL support')
    base = ['cc', '-std=c11', '-O3', '-pthread', '-Wall', '-Wextra', '-Werror']
    variants = {'generic': [], 'scalar': ['-fno-tree-vectorize'], 'native': ['-march=native'],
                'avx2': ['-mavx2'], 'avx512': ['-mavx512f', '-mavx512bw', '-mavx512vl', '-mprefer-vector-width=512']}
    commands = []
    for name, flags in variants.items():
        command = base + flags + [f'-fopt-info-vec-all={directory / (name + ".vector.txt")}',
                                 str(SOURCE / 'c-review-conversion.c'), '-o', str(directory / name)]
        run(command)
        commands.append(command)
    packages = ['libavcodec', 'libavutil', 'libva', 'libdrm', 'x11', 'x11-xcb', 'xrandr', 'xcb-dri3']
    flags = shlex.split(run(['pkg-config', '--cflags', *packages]))
    libraries = shlex.split(run(['pkg-config', '--libs', 'x11', 'x11-xcb', 'xrandr', 'xcb-dri3', 'xext']))
    command = base + ['-D_POSIX_C_SOURCE=200809L', '-ffunction-sections', '-fdata-sections',
                      '-Wl,--gc-sections', *shlex.split(os.environ.get('GPU_CFLAGS', '')), *flags,
                      str(SOURCE / 'c-review-gpu.c'), str(ROOT / 'host/gpu/options.c'),
                      '-o', str(directory / 'gpu'), *libraries]
    run(command)
    commands.append(command)
    return commands


def cases():
    # Counterbalanced comparisons; one process/session at a time.
    for scale in (1, 2):
        for kind in range(6):
            yield 'tiles', scale, kind, 1, [('generic', 0), ('generic', 1)]
        for workers in (4, 8):
            for kind in range(5):
                yield 'scheduling', scale, kind, workers, [('generic', 0), ('generic', 2)]
        for kind in (1, 2, 4):
            yield 'simd', scale, kind, 1, [(name, 0) for name in ('generic', 'scalar', 'native', 'avx2', 'avx512')]


def conversion_trial(directory, trial):
    results = []
    for group, scale, kind, workers, variants in cases():
        if trial % 2:
            variants = list(reversed(variants))
        for compiler, variant in variants:
            raw = run([str(directory / compiler), str(scale), str(kind), str(workers), str(variant)])
            for line in raw.splitlines():
                results.append(dict(json.loads(line), group=group, compiler=compiler, trial=trial))
    return results


def gpu_trial(directory, trial):
    results = []
    for identity in (0, 1):
        command = ['xvfb-run', '-a', '-s', '-screen 0 128x128x24', str(directory / 'gpu'), str(identity)]
        environment = dict(os.environ, BLENT_C_REVIEW_ISOLATED='1')
        results.append(dict(json.loads(run(command, environment)), group='metadata', trial=trial))
    return results


def metadata(commands):
    sources = list((ROOT / 'host/evdi').glob('*.[ch]')) + list((ROOT / 'host/gpu').glob('*.[ch]'))
    sources += list(SOURCE.glob('c-review-*')) + [SOURCE / 'checked_arguments.h']
    cpu = next(line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines()
               if line.startswith('model name'))
    return dict(revision=run(['git', 'rev-parse', 'HEAD']).strip(), kernel=platform.release(),
                cpu=cpu, affinity=sorted(os.sched_getaffinity(0)), compiler=run(['cc', '--version']).splitlines()[0],
                load=os.getloadavg(), commands=commands,
                source_sha256={str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
                limitations=['Single host synthetic workloads, not end-to-end latency or physical GPU acceptance.',
                             'Tiles and weighted scheduling are unshipped hypotheses.',
                             'Every conversion process checks an independent full-frame scalar pixel oracle.',
                             'SIMD native binary is local-only, not portable release output.'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--trials', type=int, choices=range(1, 10), default=3)
    args = parser.parse_args()
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    commands = build(directory)
    (directory / 'metadata.json').write_text(json.dumps(metadata(commands), indent=2) + '\n')
    results = []
    for trial in range(args.trials):
        results.extend(conversion_trial(directory, trial))
        results.extend(gpu_trial(directory, trial))
        (directory / 'raw.json').write_text(json.dumps(results, indent=2) + '\n')
        print(f'Trial {trial + 1}/{args.trials} complete', flush=True)


if __name__ == '__main__':
    main()
