#!/usr/bin/env python3
"""Model the extraction ownership contract; actual runtime qualification is separate."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

source = Path(os.environ['BLENT_TEST_TEMPLATE'])
with tempfile.TemporaryDirectory(prefix='blent-runtime-') as root:
    app = Path(root) / 'AppDir'
    shutil.copytree(source, app)
    env = dict(os.environ, APPIMAGE=os.path.realpath(__file__), APPDIR=str(app))
    result = subprocess.run([app / 'AppRun', *sys.argv[1:]], env=env)
    raise SystemExit(result.returncode)
