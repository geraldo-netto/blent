#!/usr/bin/env python3
"""T880: collect offline Rust coverage in a fresh, exclusively owned target tree."""
import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import subprocess

from model import fingerprint, verify_sources
from report import ROOT, snapshot


@dataclass(frozen=True)
class Selection:
    packages: tuple = ()
    tests: tuple = ()
    library: bool = False
    all_features: bool = False
    filter: str | None = None
    features: tuple = ()


def names(option, values):
    for value in values:
        if not isinstance(value, str) or not re.fullmatch(r'[A-Za-z0-9_][A-Za-z0-9_.-]{0,255}', value):
            raise ValueError(f'invalid {option} name')
        yield f'--{option}={value}'


def command(selection, report):
    args = ['cargo', 'llvm-cov', 'test', '--offline', '--locked', '--lcov', '--output-path', str(report)]
    args.extend(names('package', selection.packages))
    args.extend(names('test', selection.tests))
    args.extend(names('features', selection.features))
    if not selection.packages:
        args.append('--workspace')
    if selection.library:
        args.append('--lib')
    if selection.all_features:
        args.append('--all-features')
    if selection.filter is not None:
        if not isinstance(selection.filter, str) or '\x00' in selection.filter:
            raise ValueError('invalid test filter')
        args.extend(['--', selection.filter])
    return args


def environment(target, inherited):
    env = dict(inherited)
    env.pop('LLVM_PROFILE_FILE', None)
    env.update(CARGO_NET_OFFLINE='true', CARGO_TARGET_DIR=str(target),
               CARGO_LLVM_COV_TARGET_DIR=str(target), CARGO_LLVM_COV_BUILD_DIR=str(target),
               CARGO_BUILD_BUILD_DIR=str(target / 'build'))
    return env


def native_object(path):
    with path.open('rb') as source:
        header = source.read(4)
    return header == b'\x7fELF' or header[:2] == b'MZ'


def owned_objects(target):
    objects = {}
    for path in sorted(target.rglob('*')):
        if not path.is_file() or not native_object(path):
            continue
        if path.is_symlink() or not path.resolve().is_relative_to(target.resolve()):
            raise ValueError('native coverage object escaped the owned target tree')
        objects[str(path.relative_to(target.parent))] = fingerprint(path)
    if not objects:
        raise ValueError('collection produced no owned native objects')
    return objects


def validate_collection(root, manifest, snapshotter, output):
    verify_sources(root, manifest['sources'])
    if snapshotter(root) != manifest:
        raise ValueError('source inventory changed during collection')
    report = output / 'rust.lcov'
    if not report.is_file() or not report.stat().st_size:
        raise ValueError('collection produced no native LCOV report')
    return owned_objects(output / 'target')


def collect(root, output, selection, runner=subprocess.run, snapshotter=snapshot, inherited=None):
    root, output = root.resolve(), output.absolute()
    args = command(selection, output / 'rust.lcov')
    # Refuse reuse before any Cargo action. Keep failed evidence for diagnosis;
    # never clean a caller's preexisting target or silently reuse its mappings.
    output.mkdir(parents=True, exist_ok=False)
    target = output / 'target'
    target.mkdir()
    manifest = snapshotter(root)
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    with (output / 'collection.log').open('w') as log:
        runner(args, cwd=root, env=environment(target, os.environ if inherited is None else inherited),
               stdout=log, stderr=subprocess.STDOUT, check=True)
    objects = validate_collection(root, manifest, snapshotter, output)
    proof = dict(version=1, command=args, sources=manifest['sources'], objects=objects,
                 report_sha256=fingerprint(output / 'rust.lcov'), collector_sha256=fingerprint(Path(__file__)))
    (output / 'attestation.json').write_text(json.dumps(proof, indent=2) + '\n')
    return proof


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path, help='new directory; never reuse an existing path')
    parser.add_argument('--package', action='append', default=[])
    parser.add_argument('--test', action='append', default=[])
    parser.add_argument('--lib', action='store_true')
    parser.add_argument('--all-features', action='store_true')
    parser.add_argument('--feature', action='append', default=[], help='explicit feature name; repeat as needed')
    parser.add_argument('--filter')
    args = parser.parse_args()
    collect(ROOT, args.output, Selection(tuple(args.package), tuple(args.test), args.lib,
                                       args.all_features, args.filter, tuple(args.feature)))


if __name__ == '__main__':
    main()
