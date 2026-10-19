"""Scoped offline audit: cached pinned Rust grammar; repository counting functions unchanged."""
import ast
import ctypes
from dataclasses import asdict
import importlib.metadata
import json
from pathlib import Path
import sys
import types

ROOT = Path.cwd().resolve()
sys.path.insert(0, '/home/netto/.cache/uv/archive-v0/nOds33o--6Q7Ze_u')
sys.path.insert(0, str(ROOT / 'scripts/complexity'))
sys.path.insert(0, str(ROOT / 'scripts/coverage'))
from tree_sitter import Language, Parser

native = ctypes.CDLL('/tmp/blent-t1049-evidence/parsers/rust.so')
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

FILES = ['host/src/accessory/setup.rs','host/src/broker/link.rs',
         'host/src/broker/linux/entry.rs','host/src/broker/linux/server/settings.rs']
TESTS = ['host/src/broker/link/tests.rs','host/src/broker/link/tests/handshake.rs',
         'host/src/broker/link/tests/handshake/deadline.rs','host/src/broker/linux/entry/tests.rs',
         'host/src/broker/linux/server/settings/tests.rs','host/src/broker/linux/server/tests/identity.rs',
         'host/src/broker/linux/server/tests/tls.rs']
BASE = Path('/tmp/blent-t1105-evidence')

def snapshot(root):
    paths = [path for path in source_files(root) if path.suffix == '.rs']
    hashes = {str(path): fingerprint(root / path) for path in paths}
    functions = [asdict(function) for name in FILES
                 for function in syntax_functions(Path(name), (root / name).read_text(), 'rust')]
    return dict(version=1, sources=hashes, build_sources=hashes, functions=functions)

scores = [dict(file=name, line=line, name=function, complexity=score)
          for name in FILES + TESTS
          for line, function, score in module.syntax_scores((ROOT / name).read_text(), 'rust')]
(BASE/'complexity.json').write_text(json.dumps(dict(
    tool='repository source audit; not native SonarQube',
    parser='tree-sitter-rust 0.24.2; tree-sitter 0.25.2', functions=scores),indent=2)+'\n')
assert all(row['complexity'] <= 9 for row in scores), [row for row in scores if row['complexity'] > 9]
print('Source audit:',len(scores),'functions, max',max(row['complexity'] for row in scores),flush=True)
if sys.argv[1] == 'audit':
    sys.exit(0)
output=BASE/sys.argv[1]
collect(ROOT,output,Selection(packages=('blent','blent-config'),tests=('usb_broker',),library=True,features=('accessory-transport',),filter=None),snapshotter=snapshot)
manifest=json.loads((output/'manifest.json').read_text());verify_sources(ROOT,manifest['sources'])
data={};lcov(output/'rust.lcov',ROOT,data)
rows=[result(Function(**entry),data) for entry in manifest['functions']]
(output/'per-function.json').write_text(json.dumps(dict(passes=all(row['passes'] for row in rows),functions=rows),indent=2)+'\n')
for row in rows:
    print(row['name'],row['percent'],row['missing'],flush=True)
assert rows and all(row['passes'] for row in rows),'per-function below 80% or unmeasured'
