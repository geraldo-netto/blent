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
BASELINE = 'a3c053e'


def run(command, environment=None):
    return subprocess.check_output(command, cwd=ROOT, text=True, stderr=subprocess.PIPE, env=environment)


def snapshot(directory):
    """The original experiment uses private conversion helpers; pin their ABI."""
    paths = run(['git', 'ls-tree', '-r', '--name-only', BASELINE, 'host/evdi', 'host/gpu']).splitlines()
    paths = [path for path in paths if path.endswith(('.c', '.h'))]
    paths += ['scripts/benchmarks/' + name for name in
              ('c-review-conversion.c', 'c-review-gpu.c', 'checked_arguments.h')]
    target = directory / 'baseline'
    for path in paths:
        destination = target / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(subprocess.check_output(['git', 'show', f'{BASELINE}:{path}'], cwd=ROOT))
    return target


def build(directory, snapshot_root):
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
                                 str(snapshot_root / 'scripts/benchmarks/c-review-conversion.c'), '-o', str(directory / name)]
        run(command)
        commands.append(command)
    packages = ['libavcodec', 'libavutil', 'libva', 'libdrm', 'x11', 'x11-xcb', 'xrandr', 'xcb-dri3']
    flags = shlex.split(run(['pkg-config', '--cflags', *packages]))
    libraries = shlex.split(run(['pkg-config', '--libs', 'x11', 'x11-xcb', 'xrandr', 'xcb-dri3', 'xext']))
    command = base + ['-D_POSIX_C_SOURCE=200809L', '-ffunction-sections', '-fdata-sections',
                      '-Wl,--gc-sections', *shlex.split(os.environ.get('GPU_CFLAGS', '')), *flags,
                      str(snapshot_root / 'scripts/benchmarks/c-review-gpu.c'), str(snapshot_root / 'host/gpu/options.c'),
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


def metadata(commands, snapshot_root):
    sources = list(snapshot_root.rglob('*.c')) + list(snapshot_root.rglob('*.h'))
    cpu = next(line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines()
               if line.startswith('model name'))
    return dict(revision=run(['git', 'rev-parse', BASELINE]).strip(), kernel=platform.release(),
                cpu=cpu, affinity=sorted(os.sched_getaffinity(0)), compiler=run(['cc', '--version']).splitlines()[0],
                load=os.getloadavg(), commands=commands,
                source_sha256={str(p.relative_to(snapshot_root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
                runner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
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
    snapshot_root = snapshot(directory)
    commands = build(directory, snapshot_root)
    (directory / 'metadata.json').write_text(json.dumps(metadata(commands, snapshot_root), indent=2) + '\n')
    results = []
    for trial in range(args.trials):
        results.extend(conversion_trial(directory, trial))
        results.extend(gpu_trial(directory, trial))
        (directory / 'raw.json').write_text(json.dumps(results, indent=2) + '\n')
        print(f'Trial {trial + 1}/{args.trials} complete', flush=True)


if __name__ == '__main__':
    main()
