"""T1075: pure report checks do not require source-discovery parsers."""
import os
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
GUARD = '''
import importlib.abc
import sys
from pathlib import Path
class BlockInventory(importlib.abc.MetaPathFinder):
    def find_spec(self, fullname, path=None, target=None):
        if fullname == 'inventory':
            raise RuntimeError('T1075 source parser requested')
sys.meta_path.insert(0, BlockInventory())
from report import collect, sources, build_source_paths, snapshot
from model import Function
from dataclasses import asdict
'''


class ReportImportsTest(unittest.TestCase):
    def run_fixture(self, program):
        environment = dict(os.environ, PYTHONPATH=str(ROOT/'scripts/coverage'))
        result = subprocess.run([sys.executable, '-c', GUARD+program], cwd=ROOT,
                                env=environment, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stdout+result.stderr)

    def test_t1075_already_captured_native_counters_need_no_source_parsers(self):
        self.run_fixture('''
function = Function('A.kt', 'run', 1, 2, 'kotlin')
method = dict(file='A.kt', name='run', line=1, covered=2, total=2)
rows = collect({'functions': [asdict(function)]}, {}, ([method], {}), [])
assert len(rows) == 1 and rows[0]['passes']
''')

    def test_t1075_source_operations_still_require_their_real_inventory(self):
        self.run_fixture('''
for operation in [sources, build_source_paths, snapshot]:
    try:
        operation(Path('.'))
    except RuntimeError as failure:
        assert str(failure) == 'T1075 source parser requested'
    else:
        raise AssertionError('T1075 missing parser silently bypassed inventory')
''')
