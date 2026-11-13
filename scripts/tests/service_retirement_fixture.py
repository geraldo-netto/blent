"""T1412 disposable service: only its parent may authorize child retirement."""
from pathlib import Path
import signal
import subprocess
import sys
import time


def wait_for(path):
    while not path.exists():
        time.sleep(0.01)


def child(root):
    signal.signal(signal.SIGTERM, lambda *_: (root / 'child-signalled').touch())
    (root / 'child-ready').touch()
    wait_for(root / 'child-release')


def parent(root):
    if (root / 'settled').exists():
        (root / 'replacement').touch()
        return
    signal.signal(signal.SIGTERM, lambda *_: (root / 'stopping').touch())
    with subprocess.Popen([sys.executable, __file__, str(root), 'child']) as worker:
        wait_for(root / 'child-ready')
        (root / 'ready').touch()
        wait_for(root / 'release')
        (root / 'child-release').touch()
        worker.wait()
    (root / 'settled').touch()


if __name__ == '__main__':
    root = Path(sys.argv[1])
    if len(sys.argv) == 3:
        child(root)
    else:
        parent(root)
