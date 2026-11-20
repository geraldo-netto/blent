"""T1449: actual repeated GUI launch in private PID/net/mount/display namespaces."""
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from Xlib import display
import process_exit


def relaunch(binary, env, original, connection):
    while not process_exit.owns_window(connection, original.pid):
        assert original.poll() is None
        time.sleep(0.01)
    with subprocess.Popen([binary], env=env, stdin=subprocess.DEVNULL) as second:
        try:
            while second.poll() is None:
                assert not process_exit.owns_window(connection, second.pid), 'T1449 duplicate GUI window'
                time.sleep(0.01)
            assert second.returncode == 0
            assert original.poll() is None
            assert process_exit.owns_window(connection, original.pid)
        finally:
            if second.poll() is None:
                second.kill()
            second.wait()


def run(binary):
    with tempfile.TemporaryDirectory(prefix='blent-t1449-') as directory:
        root = Path(directory)
        process_exit.isolate_devices(root)
        env = process_exit.environment(root)
        connection = display.Display()
        original = subprocess.Popen([binary], env=env, stdin=subprocess.DEVNULL)
        try:
            relaunch(binary, env, original, connection)
            process_exit.close_owned(original)
            assert original.wait() == 0
            print('PASS second launch exited; original window survived; owned GUI closed', flush=True)
        finally:
            connection.close()
            if original.poll() is None:
                original.kill()
            original.wait()


if __name__ == '__main__':
    run(sys.argv[1])
