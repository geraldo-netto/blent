"""Independent native POSIX lock probe for one original private test inode."""
import errno
import fcntl
import os
import stat
import sys


def identity(values):
    if len(values) != 4 or any(len(value) > 20 for value in values[2:]):
        return None
    if len(values[1]) > 4096:
        return None
    try:
        expected = tuple(map(int, values[2:]))
    except ValueError:
        return None
    return expected if all(0 <= value < 1 << 64 for value in expected) else None


def main():
    expected = identity(sys.argv)
    if expected is None:
        return 2
    try:
        descriptor = os.open(sys.argv[1], os.O_RDWR | os.O_NOFOLLOW)
    except OSError:
        return 2

    try:
        actual = os.fstat(descriptor)
        if (actual.st_dev, actual.st_ino) != expected:
            return 2
        if not stat.S_ISREG(actual.st_mode) or actual.st_uid != os.getuid():
            return 2
        try:
            fcntl.lockf(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as failure:
            return 1 if failure.errno in (errno.EAGAIN, errno.EACCES) else 2
        return 0
    finally:
        os.close(descriptor)


if __name__ == "__main__":
    raise SystemExit(main())
