#!/usr/bin/env python3
"""T418: conversion-to-encoded-packet latency using actual production adapters.

No EVDI attach, Android activity switch, USB, or physical-display claim.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
# Copy native private adapters verbatim; use the production library for shared
# contracts so VideoOutput and MediaBytes retain their actual type identity.
FILES = ['encoder.rs', 'encoder_hardware.rs', 'encoder_frame.rs', 'encoder_shared.rs',
         'encoder_storage.rs', 'encoder_io.rs', 'encoder_fifo.rs', 'raw_memory.rs',
         'raw_socket.rs']


def target_directory(folder):
    return Path(os.environ.get('CARGO_TARGET_DIR', folder / 'target')).resolve()


def build(folder):
    (folder / 'src').mkdir(parents=True, exist_ok=True)
    manifest = {}
    for name in FILES:
        data = (ROOT / 'host/src' / name).read_bytes()
        (folder / 'src' / name).write_bytes(data)
        manifest[name] = hashlib.sha256(data).hexdigest()
    with (folder / 'src/encoder.rs').open('a') as source:
        source.write((ROOT / 'scripts/benchmarks/shared-encode.rs').read_text())
    modules = ['encoder', 'encoder_io', 'raw_memory', 'raw_socket']
    (folder / 'src/main.rs').write_text('\n'.join('mod ' + name + ';' for name in modules) +
        '\nuse blent::{annex_scan, config, latency, media, media_storage};\nfn main() { encoder::benchmark(&std::env::args().skip(1).collect::<Vec<_>>()).unwrap(); }\n')
    (folder / 'Cargo.toml').write_text('''[package]
name = "blent-shared-encode-bench"
version = "0.0.0"
edition = "2021"
[features]
default = ["inproc-encoder"]
inproc-encoder = []
[dependencies]
anyhow = "1"
bytes = "1"
libc = "0.2"
serde_json = "1"
tempfile = "3"
tracing = "0.1"
tokio = { version = "1", features = ["full"] }
ffmpeg-next = "9"
blent-config = { path = ''' + json.dumps(str(ROOT / 'common')) + ' }\n' +
        'blent = { path = ' + json.dumps(str(ROOT / 'host')) +
        ', features = ["inproc-encoder"] }\n')
    # Reuse the project dependency versions; resolution may only use local cache.
    shutil.copyfile(ROOT / 'Cargo.lock', folder / 'Cargo.lock')
    subprocess.run(['cargo', 'build', '--offline', '--release',
                    '--target-dir', str(target_directory(folder))], cwd=folder, check=True)
    subprocess.run(['cc', '-O3', '-pthread', '-I', str(ROOT / 'host/evdi'),
        str(ROOT / 'scripts/benchmarks/shared-encode-producer.c'),
        str(ROOT / 'host/evdi/raw_ring.c'), str(ROOT / 'host/evdi/conversion.c'), str(ROOT / 'host/evdi/cursor_overlay.c'),
        '-o', str(folder / 'producer')], check=True)
    (folder / 'source-hashes.json').write_text(json.dumps(manifest, indent=2) + '\n')


def run(args):
    binary = target_directory(args.directory) / 'release/blent-shared-encode-bench'
    rows = []
    for repeat in range(args.repeats):
        for mode in (['fifo', 'shared'] if repeat % 2 == 0 else ['shared', 'fifo']):
            prefix = args.directory / f'{repeat}-{mode}'
            with prefix.with_suffix('.log').open('w') as log:
                output = subprocess.check_output([str(binary), str(args.directory / 'producer'), mode,
                    '1280', '800', str(args.frames), str(args.workers), str(prefix.with_suffix('.h264'))],
                    stderr=log, text=True, timeout=60)
            row = json.loads(output)
            row['repeat'] = repeat
            prefix.with_suffix('.json').write_text(json.dumps(row, indent=2) + '\n')
            rows.append(row)
            args.output.write_text(json.dumps(dict(affinity=sorted(os.sched_getaffinity(0)), trials=rows), indent=2) + '\n')
            print(repeat, mode, len(row['frames']), 'frames', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--frames', type=int, default=300)
    parser.add_argument('--repeats', type=int, default=5)
    parser.add_argument('--workers', type=int, default=30)
    parser.add_argument('--reuse', action='store_true')
    parser.add_argument('--build-only', action='store_true')
    args = parser.parse_args()
    if not (1 <= args.frames <= 600 and 1 <= args.repeats <= 10 and 1 <= args.workers <= 128):
        parser.error('bounded positive frame, repeat and worker counts required')
    if not args.reuse:
        build(args.directory)
    if not args.build_only:
        run(args)


if __name__ == '__main__':
    main()
