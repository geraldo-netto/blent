"""Current native EDID CLI fixture; contains no encoder implementation."""
from functools import lru_cache
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


@lru_cache(maxsize=1)
def binary():
    target = ROOT / 'target' / 'edid-tool-tests'
    env = os.environ.copy()
    for name in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'LLVM_PROFILE_FILE']:
        env.pop(name, None)
    subprocess.run(['cargo', 'build', '--offline', '--locked', '-p', 'blent-config',
                    '--bin', 'blent-edid', '--target-dir', str(target)],
                   cwd=ROOT, env=env, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return target / 'debug' / ('blent-edid.exe' if os.name == 'nt' else 'blent-edid')
