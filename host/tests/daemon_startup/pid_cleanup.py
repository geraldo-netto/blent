#!/usr/bin/python3
"""T1428: failed startup retires its PID in the fake-only namespace fixture."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

from signals import configure, reap


def failed_startup(binary):
    with tempfile.TemporaryDirectory(prefix='blent-t1428-') as directory:
        root = Path(directory)
        env, _ = configure(root)
        env['BLENT_ACTIVITY_LIMIT'] = '0'
        child = subprocess.Popen([binary, '--helper', '/bin/true', 'start'], env=env,
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            stdout, stderr = child.communicate()
            pid = root / '.local/share/blent/blent.pid'
            result = dict(exit=child.returncode, pid_file_exists=pid.exists())
            print(json.dumps(result), flush=True)
            assert child.returncode == 1, (stdout, stderr)
            assert 'activity admission: Resource limit is out of range' in stderr, stderr
            assert not pid.exists(), 'T1428: failed startup left its PID published'
        finally:
            reap(child)


def main():
    assert os.getpid() == 1, 'Must run in isolated PID namespace'
    subprocess.run(['/bin/mount', '-t', 'tmpfs', 'tmpfs', '/dev'], check=True)
    Path('/dev/null').touch()
    failed_startup(str(Path(sys.argv[1]).resolve()))


if __name__ == '__main__':
    main()
