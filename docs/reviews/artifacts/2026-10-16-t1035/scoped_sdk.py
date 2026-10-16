"""Scoped offline audit: cached pinned Rust grammar; repository counting functions unchanged."""
import ast
import ctypes
from dataclasses import asdict
import importlib.metadata
import json
from pathlib import Path
import sys
import subprocess
import os
import tempfile
import types

ROOT = Path.cwd().resolve()
sys.path.insert(0, '/home/netto/.cache/uv/archive-v0/nOds33o--6Q7Ze_u')
sys.path.insert(0, str(ROOT / 'scripts/complexity'))
sys.path.insert(0, str(ROOT / 'scripts/coverage'))
from tree_sitter import Language, Parser

native = ctypes.CDLL('/tmp/blent-t923-evidence/parsers/rust.so')
native.tree_sitter_rust.restype = ctypes.c_void_p
capsule = ctypes.pythonapi.PyCapsule_New
capsule.restype = ctypes.py_object
capsule.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p]
parser = Parser(Language(capsule(native.tree_sitter_rust(), b'tree_sitter.Language', None)))
# Only Rust is in scope. Initialize its pinned cached parser directly; retain
# every source-counting definition verbatim, without unavailable C/Shell imports.
path = ROOT / 'scripts/complexity/syntax.py'
tree = ast.parse(path.read_text())
tree.body = [node for node in tree.body if not isinstance(node, (ast.Import, ast.ImportFrom))
             and not (isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'PARSERS' for t in node.targets))]
module = types.ModuleType('syntax')
module.__dict__.update(ast=ast, PARSERS={'rust': parser})
exec(compile(tree, str(path), 'exec'), module.__dict__)
sys.modules['syntax'] = module
from check import source_files
from inventory import syntax_functions
from model import Function, fingerprint, result, verify_sources
from readers import lcov
from rust_collect import Selection, collect

FILES = ['host/src/capture/helper.rs', 'host/src/capture/helper/activity.rs']
TESTS = ['host/src/capture/helper/activity_tests.rs']
SELECTED = {'host/src/capture/helper.rs': ['new', 'enable_activity', 'set_capture_active', 'start_with', 'stop', 'terminate']}
BASE = Path('/tmp/blent-t1035-evidence')

def snapshot(root):
    paths = [path for path in source_files(root) if path.suffix == '.rs']
    hashes = {str(path): fingerprint(root / path) for path in paths}
    functions = [asdict(function) for name in FILES
                 for function in syntax_functions(Path(name), (root / name).read_text(), 'rust')
                 if name not in SELECTED or function.name in SELECTED[name]]
    return dict(version=1, sources=hashes, build_sources=hashes, functions=functions)

scores = [dict(file=name, line=line, name=function, complexity=score)
          for name in FILES + TESTS
          for line, function, score in module.syntax_scores((ROOT / name).read_text(), 'rust')
          if name not in SELECTED or function in SELECTED[name]]
(BASE / 'complexity.json').write_text(json.dumps(dict(
    tool='repository Sonar-rule source audit; not native SonarQube',
    parser='tree-sitter-rust 0.24.2 (cached crate), tree-sitter 0.25.2',
    functions=scores), indent=2) + '\n')
assert all(row['complexity'] <= 9 for row in scores), [row for row in scores if row['complexity'] > 9]
print(f"Source complexity: {len(scores)} functions, max {max(row['complexity'] for row in scores)}", flush=True)
if sys.argv[1] == 'audit':
    sys.exit(0)
def sdk_runner(args, **kwargs):
    subprocess.run(['cargo', 'llvm-cov', 'test', '--offline', '--locked', '-p', 'blent',
        '--bin', 'blent', '--features', 'accessory-transport,inproc-encoder',
        '--no-report', '--', '--list'], **kwargs)
    target = Path(kwargs['env']['CARGO_TARGET_DIR'])
    sdk = Path('/home/netto/.local/share/flatpak/runtime/org.freedesktop.Sdk/x86_64/25.08/active/files')
    loader = [str(sdk / 'lib64/ld-linux-x86-64.so.2'), '--library-path',
        f'{sdk}/lib/x86_64-linux-gnu:{sdk}/lib/x86_64-linux-gnu/pulseaudio']
    binaries = [path for path in (target / 'debug/deps').glob('blent-*')
                if path.is_file() and path.read_bytes()[:4] == b'\x7fELF']
    assert len(binaries) == 1, binaries
    listing = subprocess.run(loader + [str(binaries[0]), '--list'],
        env=dict(kwargs['env'], LLVM_PROFILE_FILE=str(target / 'blent-list-%p-%m.profraw')),
        stdout=subprocess.PIPE, stderr=kwargs['stderr'], check=True).stdout.decode()
    tests = [line.removesuffix(': test') for line in listing.splitlines()
             if line.startswith('capture::helper::activity_tests::t1035')]
    assert len(tests) == 8, tests
    for test in tests:
        with tempfile.TemporaryDirectory(prefix='blent-t1035-sdk-') as directory:
            root = Path(directory)
            root.chmod(0o700)
            env = dict(kwargs['env'], BLENT_T429_RUNTIME='1', HOME=directory,
                XDG_RUNTIME_DIR=directory, XDG_CONFIG_HOME=str(root / 'config'),
                XDG_STATE_HOME='t1035-no-native-broker',
                DBUS_SESSION_BUS_ADDRESS='unix:path=/missing-t1035-bus',
                LLVM_PROFILE_FILE=str(target / 'blent-%p-%m.profraw'))
            for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'BLENT_FAKE_TABLET', 'LD_LIBRARY_PATH']:
                env.pop(key, None)
            subprocess.run(['/usr/bin/unshare', '--user', '--map-root-user', '--mount', '--pid',
                '--fork', '--mount-proc'] + loader + [str(binaries[0]), '--exact', test, '--nocapture', '--test-threads=1'],
                cwd=kwargs['cwd'], env=env, stdout=kwargs['stdout'], stderr=kwargs['stderr'], check=True)
    subprocess.run(['cargo', 'llvm-cov', 'report', '--lcov', '--output-path', str(target.parent / 'rust.lcov')], **kwargs)

output = BASE / sys.argv[1]
if not output.exists():
    collect(ROOT, output, Selection(packages=('blent',), binaries=True, features=('accessory-transport','inproc-encoder'), filter='t1035'), runner=sdk_runner, snapshotter=snapshot)
manifest = json.loads((output / 'manifest.json').read_text())
assert json.loads(json.dumps(snapshot(ROOT))) == manifest
verify_sources(ROOT, manifest['sources'])
data = {}
lcov(output / 'rust.lcov', ROOT, data)
rows = [result(Function(**entry), data) for entry in manifest['functions']]
(output / 'per-function.json').write_text(json.dumps(dict(passes=all(row['passes'] for row in rows), functions=rows), indent=2) + '\n')
for row in rows:
    print(row['file'], row['name'], row['percent'], row['missing'])
assert all(row['passes'] for row in rows), 'per-function coverage below 80% or missing'
