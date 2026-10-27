"""Offline crate notice discovery, bounded-memory copying and source binding."""
from contextlib import contextmanager
import hashlib
import os
from pathlib import Path
import shutil
import stat
import tarfile


# These font attribution files use established names without a LICENSE prefix.
FONT_NOTICES = {'ofl.txt', 'ufl.txt', 'hack-regular.txt'}
NOTICE_PREFIXES = ('license', 'licence', 'copying', 'copyright', 'notice', 'unlicense')


def notice_name(name):
    lowered = name.lower()
    return (lowered.startswith(NOTICE_PREFIXES) or lowered in FONT_NOTICES
            or '-license' in lowered or '-licence' in lowered)


def regular_relative(source, path):
    relative = path.relative_to(source)
    current = source
    for component in relative.parts:
        current /= component
        if current.is_symlink():
            raise ValueError(f'notice path contains a symlink: {relative}')
    if not path.is_file():
        raise ValueError(f'notice is missing or not a regular file: {relative}')
    return relative


def declared_notice(source, value):
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise ValueError('invalid declared notice path')
    relative = Path(value)
    if relative.is_absolute() or '..' in relative.parts:
        raise ValueError('declared notice path escapes crate')
    return regular_relative(source, source / relative)


def candidate(source, path):
    relative = path.relative_to(source)
    if not any(notice_name(part) for part in relative.parts):
        return None
    if path.is_symlink():
        raise ValueError(f'notice path contains a symlink: {relative}')
    if path.is_dir():
        return None
    return regular_relative(source, path)


def notice_paths(source, license_file):
    if not source.is_dir():
        raise ValueError('crate notice source is not a directory')
    declared = declared_notice(source, license_file)
    selected = set()
    for path in source.rglob('*'):
        relative = candidate(source, path)
        if relative is not None:
            selected.add(relative)
    if declared is not None:
        selected.add(declared)
    if not selected:
        raise ValueError(f'no crate notice files found in {source}; supply the missing upstream notices before packaging')
    return sorted(selected)


@contextmanager
def regular_reader(path):
    # Refuse known devices/FIFOs before opening; a raced FIFO cannot block open.
    if not stat.S_ISREG(path.stat().st_mode):
        raise ValueError('notice input must be a regular file')
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
    try:
        with os.fdopen(descriptor, 'rb', closefd=False) as stream:
            if not stat.S_ISREG(os.fstat(descriptor).st_mode):
                raise ValueError('notice input changed from a regular file')
            yield stream
    finally:
        os.close(descriptor)


def copy_notice(source, target):
    checksum = hashlib.sha256()
    meaningful = False
    target.parent.mkdir(parents=True, exist_ok=True)
    with regular_reader(source) as reader, target.open('xb') as writer:
        for chunk in iter(lambda: reader.read(65536), b''):
            meaningful = meaningful or bool(chunk.strip())
            checksum.update(chunk)
            writer.write(chunk)
    if not meaningful:
        raise ValueError(f'notice contains no text: {source}')
    return checksum.hexdigest()


def copy_crate_notices(source, destination, license_file):
    paths = notice_paths(source, license_file)
    # Exclusive creation establishes ownership; never remove an existing output.
    destination.mkdir()
    try:
        return [dict(path=path.as_posix(), sha256=copy_notice(source / path, destination / path))
                for path in paths]
    except BaseException:
        shutil.rmtree(destination)
        raise


def verify_archive_notices(path, prefix, notices):
    with tarfile.open(path) as archive:
        for notice in notices:
            member = archive.getmember(f'{prefix}/{notice["path"]}')
            if not member.isfile():
                raise ValueError('archived notice is not a regular file')
            with archive.extractfile(member) as stream:
                checksum = hashlib.file_digest(stream, 'sha256').hexdigest()
            if checksum != notice['sha256']:
                raise ValueError('crate notice changed while archiving sources')
