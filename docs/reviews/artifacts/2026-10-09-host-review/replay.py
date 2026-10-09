#!/usr/bin/env python3
"""Offline owner fault injection; no real devices or installed app operations."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def main():
    names = ['camera_control.rs', 'audio_control.rs']
    sources = {name: (ROOT / 'host/src' / name).read_bytes() for name in names}
    with tempfile.TemporaryDirectory(prefix='blent-host-review-') as folder:
        path = Path(folder)
        (path / 'Cargo.lock').write_bytes((ROOT / 'Cargo.lock').read_bytes())
        (path / 'Cargo.toml').write_text('''[package]
name = "blent-host-review"
version = "0.0.0"
edition = "2021"
[[bin]]
name = "blent-host-review"
path = "main.rs"
[dependencies]
anyhow = "1"
tokio = { version = "1", features = ["full"] }
blent-config = { path = ''' + json.dumps(str(ROOT / 'common')) + ' }\n')
        for name, source in sources.items():
            (path / name).write_bytes(source)
        (path / 'main.rs').write_bytes((HERE / 'probe.rs').read_bytes())
        command = ['cargo', 'build', '--offline', '--manifest-path', str(path / 'Cargo.toml'),
                   '--target-dir', str(ROOT / 'target')]
        subprocess.run(command, cwd=ROOT, check=True, timeout=180,
                       env={**os.environ, 'CARGO_NET_OFFLINE': 'true'})
        completed = subprocess.run([str(ROOT / 'target/debug/blent-host-review')],
                                   text=True, capture_output=True, timeout=15, check=True)
    result = dict(scope='Actual host controllers; injected cooperative stalls/panics and bounded local output fixture',
                  head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                  sources={name: hashlib.sha256(source).hexdigest() for name, source in sources.items()},
                  common_commands_sha256=hashlib.sha256((ROOT / 'common/src/commands.rs').read_bytes()).hexdigest(),
                  workspace_lock_sha256=hashlib.sha256((ROOT / 'Cargo.lock').read_bytes()).hexdigest(),
                  rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                  output=completed.stdout.splitlines(), expected_panic_stderr=completed.stderr.splitlines())
    (HERE / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    print(completed.stdout, end='')


if __name__ == '__main__':
    main()
