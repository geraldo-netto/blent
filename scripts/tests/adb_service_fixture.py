"""T1465 private fake ADB: no USB, TCP ports, or existing server access."""
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import time


def reply(root, message):
    with socket.socket(socket.AF_UNIX) as client:
        client.connect(str(root / 'socket'))
        client.sendall(message)
        return client.recv(64)


def server(root):
    with socket.socket(socket.AF_UNIX) as listener:
        listener.bind(str(root / 'socket'))
        listener.listen(1)
        (root / 'server-pid').write_text(str(os.getpid()))
        while True:
            with listener.accept()[0] as client:
                message = client.recv(64)
                client.sendall(b'owned-t1465')
            if message == b'stop':
                break
    (root / 'socket').unlink()
    (root / 'server-pid').unlink()


def bootstrap(root):
    if (root / 'socket').exists():
        assert reply(root, b'ping') == b'owned-t1465'
        return
    subprocess.Popen([sys.executable, __file__, str(root), 'server'],
                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                     stderr=subprocess.DEVNULL)
    while not (root / 'server-pid').exists():
        time.sleep(0.01)


def daemon(root):
    # Model ADB's implicit creation only when the service lacks connect-only mode.
    if os.environ.get('ADB_SERVER_SOCKET') != 'tcp:127.0.0.1:5037':
        bootstrap(root)
    assert reply(root, b'ping') == b'owned-t1465'
    (root / 'daemon-pid').write_text(str(os.getpid()))
    signal.pause()


if __name__ == '__main__':
    root = Path(sys.argv[1])
    {'server': server, '--blent-start-adb-server': bootstrap, 'start': daemon}[sys.argv[2]](root)
