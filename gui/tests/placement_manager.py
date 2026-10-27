"""Own a private test window manager until the native placement probe exits."""
import os
import subprocess
import sys
import time


def wait_ready(manager):
    while subprocess.run(['wmctrl', '-m'], capture_output=True).returncode:
        if manager.poll() is not None:
            raise RuntimeError('private window manager exited before readiness')
        time.sleep(0.05)


def main():
    manager_name, *command = sys.argv[1:]
    if manager_name == 'none':
        return subprocess.call(command)
    environment = dict(os.environ, GSETTINGS_BACKEND='memory', GIO_USE_VFS='local', NO_AT_BRIDGE='1')
    manager = subprocess.Popen([manager_name, '--sm-disable', '--replace'], env=environment,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        wait_ready(manager)
        return subprocess.call(command)
    finally:
        manager.terminate()
        manager.wait()


if __name__ == '__main__':
    raise SystemExit(main())
