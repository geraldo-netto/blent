"""T1435: close an actual GUI on private Xvfb; namespace teardown contains children."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from Xlib import X, display


def isolate_devices(root):
    for name in ['uinput', 'dri']:
        target = Path('/dev') / name
        if not target.exists():
            continue
        replacement = root / name
        if target.is_dir():
            replacement.mkdir()
        else:
            replacement.touch()
        subprocess.run(['/usr/bin/mount', '--bind', str(replacement), str(target)], check=True)


def environment(root):
    config = root / 'config/blent/config.toml'
    config.parent.mkdir(parents=True)
    config.write_text('check_updates = false\ninput_touch = false\ninput_pen = false\ninput_pointer = false\n')
    tools = root / 'bin'
    tools.mkdir()
    for name, body in [('adb', "printf 'List of devices attached\\n\\n'"), ('systemctl', 'exit 1')]:
        path = tools / name
        path.write_text('#!/bin/sh\n' + body + '\n')
        path.chmod(0o700)
    env = dict(os.environ, HOME=str(root), XDG_CONFIG_HOME=str(root / 'config'),
               XDG_RUNTIME_DIR=str(root), PATH=str(tools), WINIT_UNIX_BACKEND='x11',
               LIBGL_ALWAYS_SOFTWARE='1', GSETTINGS_BACKEND='memory')
    env.pop('WAYLAND_DISPLAY', None)
    return env


def owns_window(connection, pid):
    atom = connection.intern_atom('_NET_WM_PID')
    for window in connection.screen().root.query_tree().children:
        value = window.get_full_property(atom, X.AnyPropertyType)
        if value is not None and list(value.value) == [pid]:
            return True
    return False


def close_owned(child):
    connection = display.Display()
    try:
        while not owns_window(connection, child.pid):
            assert child.poll() is None, child.returncode
            time.sleep(0.01)
        subprocess.run([sys.executable, str(Path(__file__).with_name('close_window.py')),
                        str(child.pid)], check=True)
    finally:
        connection.close()


def run(binary):
    with tempfile.TemporaryDirectory(prefix='blent-t1435-') as directory:
        root = Path(directory)
        isolate_devices(root)
        with (root / 'output').open('w') as output:
            child = subprocess.Popen([binary], env=environment(root), stdin=subprocess.DEVNULL,
                                     stdout=output, stderr=output)
            try:
                close_owned(child)
                result = child.wait()
                assert result == 0, (result, (root / 'output').read_text())
                print('PASS actual GUI entry: owned Xvfb window closed, exit=0', flush=True)
            finally:
                if child.poll() is None:
                    child.kill()
                child.wait()


if __name__ == '__main__':
    run(sys.argv[1])
