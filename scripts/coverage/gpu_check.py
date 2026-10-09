#!/usr/bin/env python3
"""T766: measure every optional GPU C function; missing execution fails closed.

Uses installed SDKs selected by pkg-config and optional GPU_CFLAGS/GPU_LIBS.
Runs policy tests and isolated Xvfb tests only; never opens a render device.
"""
import argparse
from dataclasses import asdict
import json
import os
import re
from pathlib import Path
import shlex
import shutil
import subprocess

from compiler import compile_with_profile, export_reports
from inventory import owned_functions
from model import fingerprint, result, verify_sources
from readers import gcov_json
from report import summary

ROOT = Path(__file__).resolve().parents[2]
PACKAGES = 'libavcodec libavutil libva libdrm x11 x11-xcb xrandr xfixes xdamage xrender xext xcb-dri3'.split()


def sdk_flags():
    flags = subprocess.check_output(['pkg-config', '--cflags', '--libs', *PACKAGES], text=True)
    return shlex.split(flags) + shlex.split(os.environ.get('GPU_CFLAGS', '')) + shlex.split(os.environ.get('GPU_LIBS', ''))


def cases(flags):
    sources = sorted(str(path) for path in (ROOT/'host/gpu').glob('*.c'))
    policies = [str(ROOT/name) for name in ['host/gpu/options.c', 'host/gpu/cadence.c', 'host/tests/gpu_options.c']]
    wraps = ['-Wl,--wrap=' + name for name in ['XFixesFetchRegion', 'XDamageSubtract', 'XRenderCreatePicture', 'XFixesGetCursorImage']]
    native = [str(ROOT/'host/tests/gpu_x11.c')] + [path for path in sources if not path.endswith('/main.c')]
    lifecycle = ROOT/'host/tests/gpu_lifecycle.c'
    lifecycle_wraps = ['-Wl,--wrap=' + name for name in sorted(set(re.findall(r'__wrap_([A-Za-z0-9_]+)', lifecycle.read_text())))]
    return [('options', policies, [], 0), ('native', native, wraps + flags, 0),
            ('lifecycle', [str(lifecycle)] + native[1:], lifecycle_wraps + flags, 0), ('main', sources, flags, 2)]


def exercise(output, compiler, name, sources, flags, expected):
    binary = output/(name + '-test')
    base = ['-std=c11', '-D_POSIX_C_SOURCE=200809L', '-O0', '-g', '-fsanitize=address,undefined',
            '-fno-sanitize-recover=all', '-fno-pie', '-no-pie']
    status = compile_with_profile(compiler, base + sources + flags + ['-o', str(binary)], output/'native', ROOT)
    if status:
        raise ValueError(f'{name}: compiler failed with {status}')
    command = [str(binary)]
    if name in {'native', 'lifecycle'}:
        command = ['xvfb-run', '-a', '-s', '-screen 0 128x128x24', *command]
    with (output/(name + '.log')).open('w') as log:
        run = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=30)
    if run.returncode != expected:
        raise ValueError(f'{name}: expected exit {expected}, got {run.returncode}')


def run(output):
    output = output.resolve()
    output.mkdir(parents=True)
    compiler = shutil.which('cc')
    if compiler is None:
        raise ValueError('GPU counters require installed GCC cc and gcov')
    functions = [function for function in owned_functions(ROOT) if function.file.startswith('host/gpu/')]
    sources = {str(path.relative_to(ROOT)): fingerprint(path) for path in (ROOT/'host/gpu').iterdir() if path.suffix in {'.c', '.h'}}
    (output/'manifest.json').write_text(json.dumps(dict(sources=sources, functions=[asdict(function) for function in functions]), indent=2) + '\n')
    for case in cases(sdk_flags()):
        exercise(output, compiler, *case)
    verify_sources(ROOT, sources)
    data = {}
    for path in export_reports(output/'native'):
        gcov_json(path, ROOT, data)
    rows = [result(function, data) for function in functions]
    passed = bool(rows) and all(row['passes'] for row in rows)
    report = dict(scopes=['host/gpu/'], minimum_percent=80, passes=passed, summary=summary(rows), functions=rows)
    (output/'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['summary'], indent=2))
    return int(not passed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path, help='new directory for raw counters and per-function report')
    return run(parser.parse_args().output)


if __name__ == '__main__':
    raise SystemExit(main())
