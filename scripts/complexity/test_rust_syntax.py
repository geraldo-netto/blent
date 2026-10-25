"""T1221: supported Rust syntax keeps independent metric and span oracles."""
from pathlib import Path
import unittest

from syntax import syntax_scores
from rust_source import records


class RustSyntaxTest(unittest.TestCase):
    def test_t1221_async_closures_keep_boundary_scores(self):
        for expected in range(2, 14):
            source = 'async fn sample() { let callback = async move || {\n'
            source += 'if flag { work().await; }\n' * (expected - 2)
            source += '}; }'
            self.assertEqual(list(syntax_scores(source, 'rust')), [(1, 'sample', expected)])

    def test_t1221_real_osk_source_is_measured_without_rewriting_it(self):
        source = Path(__file__).resolve().parents[2] / 'host/src/osk.rs'
        rows = list(syntax_scores(source.read_text(), 'rust'))
        self.assertIn('restore', [row[1] for row in rows])

    def test_t1221_invalid_async_syntax_still_fails_closed(self):
        for source in ['fn f() { let c = async move || {',
                       'fn f() { let c = async || if ; }', 'fn f() { let c = async |x:| {}; }']:
            with self.subTest(source=source), self.assertRaisesRegex(ValueError, 'parse error'):
                list(syntax_scores(source, 'rust'))

    def test_t1221_unicode_and_nested_functions_keep_source_locations(self):
        source = '// café 🦀\n#[allow(dead_code)]\npub async fn outer() {\n'
        source += '    fn 内側() { if flag { work(); } }\n'
        source += '    let closure = async || { while ready { work().await; } };\n}\n'
        self.assertEqual(list(syntax_scores(source, 'rust')), [(3, 'outer', 5), (4, '内側', 2)])

    def test_t1221_utf8_byte_spans_exclude_attributes(self):
        source = '// café 🦀\n#[allow(dead_code)]\npub async fn 内側() { let c = async || {}; }\n'
        row, = records(source)
        data = source.encode()
        self.assertEqual((row.first, row.last), (3, 3))
        self.assertEqual(data[row.start:row.end], 'pub async fn 内側() { let c = async || {}; }'.encode())
        self.assertEqual(row.start, len('// café 🦀\n#[allow(dead_code)]\n'.encode()))

    def test_t1221_bom_and_shebang_preserve_original_byte_offsets(self):
        function = 'fn café() { let callback = async || {}; }'
        for prefix in ['\ufeff', '#!/usr/bin/env rust-script\n', '\ufeff#!/usr/bin/env rust-script\n']:
            source = prefix + function
            row, = records(source)
            self.assertEqual((row.start, row.end), (len(prefix.encode()), len(source.encode())))
            self.assertEqual(row.first, 1 + prefix.count('\n'))
            self.assertEqual(source.encode()[row.start:row.end], function.encode())

    def test_t1221_nested_methods_and_default_bodies_keep_their_owners(self):
        source = ('fn outer() { struct Local; impl Local { fn method() { work(); } } }\n'
                  'trait Contract { fn declaration(); fn defaulted() { if flag { work(); } } }')
        self.assertEqual(list(syntax_scores(source, 'rust')),
                         [(1, 'outer', 2), (1, 'method', 1), (2, 'defaulted', 2)])

    def test_t1221_unknown_guards_and_macro_tokens_stay_opaque(self):
        source = ('#[cfg(any(windows, feature="future"))]\n'
                  'mod guarded { include!("child.rs"); fn f() { opaque!{ if nonsense ?? } } }')
        rows = records(source)
        self.assertEqual([row.kind for row in rows], ['module', 'include', 'function'])
        self.assertTrue(all(row.guards == [{'name': 'any', 'args': [
            {'name': 'windows'}, {'name': 'feature', 'value': 'future'}]}] for row in rows))
        self.assertEqual(list(syntax_scores(source, 'rust')), [(2, 'f', 1)])


if __name__ == '__main__':
    unittest.main()
