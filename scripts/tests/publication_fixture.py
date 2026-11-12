"""Private installer destinations; no real services, devices or user files."""
import os
from pathlib import Path

from shell_fixture import run

ROOT = Path(__file__).resolve().parents[2]


def destination(root, relative, kind):
    entry = root / relative
    entry.parent.mkdir(parents=True, exist_ok=True)
    if kind == 'file':
        entry.write_text('original\n')
        return entry, entry
    retained = entry if kind == 'directory' else root / 'redirect'
    retained.mkdir()
    (retained / 'retained').write_text('original\n')
    if kind == 'symlink':
        entry.symlink_to(retained, target_is_directory=True)
    return entry, retained / 'retained'


def execute(root, adapter, command, overrides=''):
    path = ROOT / ('scripts/install.sh' if adapter == 'source' else
                   'packaging/appimage/install-appimage.sh')
    source = path.read_text().removesuffix('main "$@"\n').split('\ncheck_stopped\n')[0]
    setup = '''
CONFIG_BASE=$HOME
DATA_BASE=$HOME
DEST=$HOME
SCRIPT_DIR=$HOME
TOOLS=$HOME
BIN_DIR=$HOME
LAUNCHER=$HOME/Blent.AppImage
ENTRY=$HOME/launcher
systemctl() { return 0; }
'''
    env = dict(os.environ, HOME=str(root), APPDIR=str(root), APPIMAGE=str(root / 'image'))
    return run(source + setup + overrides + '\n' + command + '\n',
               env=env, cwd=ROOT, capture_output=True, text=True)


def staging(root):
    prefixes = ('.blent.', '.image.', '.launcher.', '.gui-launcher.')
    return sorted(str(p.relative_to(root)) for p in root.rglob('*')
                  if p.name.startswith(prefixes))
