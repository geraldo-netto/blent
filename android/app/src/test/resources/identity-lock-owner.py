"""Hold one exact private test inode until the owning test sends one R byte."""
from contextlib import contextmanager
import fcntl
import os
import stat
import sys


def identity(values):
    if len(values) != 5:
        raise ValueError("argument count")
    if any(len(value) > 4096 for value in values[1:3]):
        raise ValueError("path bound")
    if any(len(value) > 20 for value in values[3:]):
        raise ValueError("identity bound")
    expected = tuple(map(int, values[3:]))
    if not all(0 <= value < 1 << 64 for value in expected):
        raise ValueError("identity range")
    return expected


@contextmanager
def locked_file(path, expected):
    descriptor = os.open(path, os.O_RDWR | os.O_NOFOLLOW)
    try:
        actual = os.fstat(descriptor)
        if (actual.st_dev, actual.st_ino) != expected:
            raise ValueError("foreign inode")
        if not stat.S_ISREG(actual.st_mode) or actual.st_uid != os.getuid():
            raise ValueError("foreign file")
        fcntl.lockf(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield
    finally:
        os.close(descriptor)


def main(values):
    expected = identity(values)
    with locked_file(values[1], expected):
        pending = values[2] + ".pending"
        with open(pending, "xb") as marker:
            marker.write(b"H")
        os.replace(pending, values[2])
        return 0 if sys.stdin.buffer.read(1) == b"R" else 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
