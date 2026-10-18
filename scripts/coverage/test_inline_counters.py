"""T1074 permanent factory-only and real retained native-counter controls."""
from dataclasses import asdict
import gzip
import json
from pathlib import Path
import tempfile
import unittest
from kotlin_report import read
from model import Function
from report import collect

ROOT = Path(__file__).resolve().parents[2]


class InlineCountersTest(unittest.TestCase):
    def rows(self, generated, callback='', functions=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root/'report.xml'
            path.write_text('''<report><package name="example">
<class name="example/A" sourcefilename="A.kt"><method name="flow" desc="()V" line="1"><counter type="LINE" covered="2" missed="0"/></method></class>
'''+generated+callback+'''
<sourcefile name="A.kt"><line nr="1" ci="1" mi="0"/><line nr="2" ci="8" mi="0"/></sourcefile>
</package></report>''')
            functions = functions or [Function('example/A.kt', 'flow', 1, 2, 'kotlin'),
                                      Function('example/A.kt', '<lambda>', 1, 1, 'kotlin')]
            return collect({'functions': [asdict(function) for function in functions]},
                           {}, read(path, root, root), [])

    def test_t1074_same_line_filtered_transform_cannot_borrow_factory_counters(self):
        rows = self.rows('<class name="example/A$flow$$inlined$map$1" sourcefilename="Emitters.kt"/>')
        self.assertFalse(all(row['passes'] for row in rows), 'T1074 factory-only coverage passed')
        self.assertTrue(all(not row['passes'] for row in rows))
        self.assertTrue(all('inline' in row['evidence'] for row in rows))

    def test_t1074_owned_callback_counters_resolve_the_body_without_excluding_it(self):
        callback = '<class name="example/A$flow$1" sourcefilename="A.kt"><method name="invoke" desc="()V" line="1"><counter type="LINE" covered="1" missed="0"/></method></class>'
        rows = self.rows('<class name="example/A$flow$$inlined$map$1" sourcefilename="Emitters.kt"/>', callback)
        self.assertEqual(len(rows), 2)
        self.assertTrue(all(row['passes'] for row in rows))

    def test_t1074_library_only_inline_and_unrelated_owners_do_not_poison_authored_methods(self):
        unrelated = '<class name="example/Other$flow$$inlined$map$1" sourcefilename="Emitters.kt"/>'
        self.assertTrue(all(row['passes'] for row in self.rows(unrelated)))
        library = '<class name="example/A$flow$$inlined$filterIsInstance$1" sourcefilename="Emitters.kt"/>'
        rows = self.rows(library, functions=[Function('example/A.kt', 'flow', 1, 2, 'kotlin')])
        self.assertEqual(len(rows), 1)
        self.assertTrue(rows[0]['passes'])

    def test_t1074_partial_class_counters_cannot_mask_a_filtered_transform_method(self):
        generated = '<class name="example/A$flow$$inlined$map$1" sourcefilename="Emitters.kt"><method name="collect" desc="()V"/><counter type="LINE" covered="1" missed="0"/></class>'
        rows = self.rows(generated)
        self.assertFalse(any(row['passes'] for row in rows))

    def test_t1074_bounded_foreign_prefixes_cannot_match_the_original_method(self):
        for prefix in ['Other$flow', 'A$other', 'A$flower', 'A$flowing', 'A$flowx']:
            generated = '<class name="example/'+prefix+'$$inlined$map$1" sourcefilename="Emitters.kt"/>'
            self.assertTrue(all(row['passes'] for row in self.rows(generated)))

    def test_t1074_malformed_inline_class_identity_is_rejected(self):
        with self.assertRaises(KeyError):
            self.rows('<class sourcefilename="Emitters.kt"/>')

    def test_t1074_nested_filtered_names_keep_the_original_enclosing_owner(self):
        rows = self.rows('<class name="example/A$flow$1$$inlined$map$1" sourcefilename="Emitters.kt"/>')
        self.assertFalse(any(row['passes'] for row in rows))

    def test_t1074_retained_t1069_native_factory_pass_is_rejected(self):
        base = ROOT/'docs/reviews/artifacts/2026-10-19-t1069/before-owned-projection'
        functions = json.loads(gzip.decompress((base/'inventory.json.gz').read_bytes()))
        chosen = [function for function in functions if function['file'].endswith('SessionCoordinator.kt')
                  and function['first'] == 98]
        self.assertEqual(len(chosen), 2)
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory)/'native.xml'
            report.write_bytes(gzip.decompress((base/'profileCoverage.xml.gz').read_bytes()))
            rows = collect({'functions': chosen}, {}, read(report, ROOT/'android/app/src/main/java', ROOT), [])
        self.assertFalse(any(row['passes'] for row in rows), 'T1074 retained counter gap passed')
        self.assertTrue(all(row.get('unmeasured_inline_bodies') for row in rows))
