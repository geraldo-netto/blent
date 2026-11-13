#!/usr/bin/python3
"""Review-only probe using existing fake-native namespace fixture."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True


def fixture():
    path = Path.cwd() / 'host/tests/daemon_startup/signals.py'
    spec = importlib.util.spec_from_file_location('private_startup', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    assert os.getpid() == 1, 'private PID namespace required'
    subprocess.run(['/bin/mount', '-t', 'tmpfs', 'tmpfs', '/dev'], check=True)
    Path('/dev/null').touch()
    helpers = fixture()
    with tempfile.TemporaryDirectory(prefix='blent-round3-pid-') as directory:
        root = Path(directory)
        env, _config = helpers.configure(root)
        env['BLENT_ACTIVITY_LIMIT'] = '0'
        env['LLVM_PROFILE_FILE'] = '/tmp/blent-lifecycle-round3-linux/pid-%p-%m.profraw'
        child = subprocess.run([str(Path(sys.argv[1]).resolve()), '--helper', '/bin/true', 'start'], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        pid = root / '.local/share/blent/blent.pid'
        print(json.dumps(dict(exit=child.returncode, stderr=child.stderr, pid_file_exists=pid.exists(), pid_file=pid.read_text() if pid.exists() else None)))


if __name__ == '__main__':
    main()
