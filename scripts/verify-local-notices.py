#!/usr/bin/env python3
"""T1278: offline, read-only identity check for a repaired local AppDir notice set."""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
import stat
from pathlib import Path

NOTICE_ROOT = Path('usr/share/doc/blent/bundled')
BINARY_ROOTS = ('usr/bin', 'usr/lib', 'usr/libexec')
MAX_MANIFEST = 8 * 1024 * 1024
MAX_ROWS = 8192


@contextmanager
def regular_file(path):
    if not stat.S_ISREG(path.stat().st_mode):
        raise ValueError('inventory input must be a regular file')
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
    try:
        with os.fdopen(descriptor, 'rb', closefd=False) as stream:
            if not stat.S_ISREG(os.fstat(descriptor).st_mode):
                raise ValueError('inventory input changed from a regular file')
            yield stream
    finally:
        os.close(descriptor)


def digest(path):
    with regular_file(path) as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def contained(root, name):
    if not isinstance(name, str) or not name:
        raise ValueError('missing relative path')
    path = Path(name)
    if path.is_absolute() or '..' in path.parts:
        raise ValueError('path escapes its owner')
    target = root / path
    if not target.resolve().is_relative_to(root.resolve()):
        raise ValueError('symlink escapes its owner')
    return target


def checked_rows(root, rows):
    if not isinstance(rows, list) or not 1 <= len(rows) <= MAX_ROWS:
        raise ValueError('empty or oversized inventory')
    names = set()
    for row in rows:
        path = contained(root, row['path'])
        if row['path'] in names:
            raise ValueError('duplicate inventory path')
        if digest(path) != row['sha256']:
            raise ValueError(f'identity changed: {row["path"]}')
        names.add(row['path'])
    return names


def executable_inventory(root):
    paths = {str(path.relative_to(root)) for folder in BINARY_ROOTS
             for path in (root / folder).rglob('*') if path.is_file()}
    if (root / 'AppRun').is_file():
        paths.add('AppRun')
    return paths


def attribution_links(manifest, notice_names):
    groups = manifest['groups']
    if not isinstance(groups, dict):
        raise ValueError('invalid attribution groups')
    for row in manifest['bundle_files']:
        if row['group'] not in groups:
            raise ValueError('binary has no attribution group')
    for group in groups.values():
        if not set(group.get('dependencies', [])) <= groups.keys():
            raise ValueError('attribution dependency is missing')
        notices = group['notices']
        if not notices or not set(notices) <= notice_names:
            raise ValueError('attribution group has missing notices')
    if manifest['unresolved'] != []:
        raise ValueError('notice recovery still has unresolved inputs')


def verify(root):
    directory = contained(root, str(NOTICE_ROOT))
    with regular_file(directory / 'manifest.json') as stream:
        data = stream.read(MAX_MANIFEST + 1)
    if len(data) > MAX_MANIFEST:
        raise ValueError('manifest exceeds capacity')
    manifest = json.loads(data)
    if manifest['schema'] != 'blent.local-notices.v1':
        raise ValueError('unsupported notice manifest')
    binaries = checked_rows(root, manifest['bundle_files'])
    if binaries != executable_inventory(root):
        raise ValueError('installed executable inventory differs from manifest')
    notices = checked_rows(directory, manifest['notice_files'])
    attribution_links(manifest, notices)
    return len(binaries), len(notices)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('appdir', type=Path)
    args = parser.parse_args()
    try:
        binaries, notices = verify(args.appdir)
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(1, f'Notice verification failed: {error}\n')
    print(f'Verified {binaries} installed files and {notices} notice files; no unresolved inputs')


if __name__ == '__main__':
    main()
