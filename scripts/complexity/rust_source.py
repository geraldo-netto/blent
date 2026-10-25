"""T1221: locked offline syn frontend with an explicit source-record contract."""
from dataclasses import dataclass
from functools import lru_cache
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1] / 'rust-source'


@dataclass(frozen=True)
class RustRecord:
    kind: str
    name: str
    first: int
    last: int
    start: int
    end: int
    guards: list
    test_only: bool
    detail: dict | None


@lru_cache(maxsize=1)
def executable():
    inputs = sorted(ROOT.glob('src/*.rs')) + [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock']
    digest = hashlib.sha256(b''.join(path.read_bytes() for path in inputs)).hexdigest()
    cache = Path(os.environ.get('XDG_CACHE_HOME') or Path.home() / '.cache') / 'blent-rust-source' / digest
    binary = cache / 'debug' / ('blent-rust-source.exe' if os.name == 'nt' else 'blent-rust-source')
    if not binary.is_file():
        subprocess.run(['cargo', 'build', '--offline', '--locked', '--manifest-path', str(ROOT / 'Cargo.toml'),
                        '--target-dir', str(cache)], check=True, stdout=subprocess.DEVNULL)
    return binary


@lru_cache(maxsize=32)
def records(source):
    result = subprocess.run([str(executable())], input=source, text=True, capture_output=True)
    if result.returncode:
        raise ValueError(result.stderr.strip())
    data = json.loads(result.stdout)
    if data['version'] != 1:
        raise ValueError('unsupported Rust source record version')
    return tuple(RustRecord(**row) for row in data['records'])
