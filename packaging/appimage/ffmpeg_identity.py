"""Native components declared by the current packaged FFmpeg launcher.

The host checks these files' current identity and hashes changed files; this
declaration is not a checksum cache or trust grant. Build owns the fresh AppDir.
At most 32 paths and 16 KiB JSON are emitted.
"""
import json
from pathlib import PurePosixPath
import stat
import unicodedata


def write(appdir):
    root = appdir / 'usr'
    files = ['libexec/ffmpeg']
    for file in (root / 'lib/blent-ffmpeg').iterdir():
        if len(files) == 32:
            raise ValueError('FFmpeg runtime identity exceeds 32 files')
        files.append('lib/blent-ffmpeg/' + file.name)
    files[1:] = sorted(files[1:])
    for name in files:
        validate(root, name)
    data = json.dumps({'schema': 1, 'files': files}, indent=2) + '\n'
    if len(data.encode()) > 16384:
        raise ValueError('FFmpeg runtime identity exceeds 16 KiB')
    (root / 'bin/ffmpeg.blent-runtime.json').write_text(data)


def validate(root, name):
    validate_name(name)
    target = (root / name).resolve(strict=True)
    if not target.is_relative_to(root.resolve()):
        raise ValueError('FFmpeg runtime file escapes its root')
    metadata = target.stat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 512 * 1024 * 1024:
        raise ValueError('FFmpeg runtime component must be regular and at most 512 MiB')


def validate_name(name):
    if len(name.encode()) > 256 or any(unicodedata.category(char) == 'Cc' for char in name):
        raise ValueError('invalid FFmpeg runtime path')
    path = PurePosixPath(name)
    if path.is_absolute() or any(part in ('', '.', '..') for part in name.split('/')):
        raise ValueError('invalid relative FFmpeg runtime path')
