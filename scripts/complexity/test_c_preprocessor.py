"""T843: header probes must preserve all branches, source offsets and errors."""
from pathlib import Path
import unittest

from check import file_scores
from syntax import parse, syntax_scores


class HeaderProbeTests(unittest.TestCase):
    def test_t843_real_benchmark_and_both_conditional_branches(self):
        root = Path(__file__).resolve().parents[2]
        rows = list(file_scores(root / 'scripts/benchmarks/shared-encode-producer.c'))
        self.assertEqual(len(rows), 10)
        producer = root / 'scripts/benchmarks/shared-encode-producer.c'
        line = next(i for i, text in enumerate(producer.read_text().splitlines(), 1)
                    if text.startswith('static void finish_shared('))
        self.assertIn((line, 'finish_shared', 2), rows)
        source = ('#if __has_include("local.h")\n'
                  'int yes(int n) { if (n) return 1; return 0; }\n'
                  '#elif __has_include(<system.h>)\n'
                  'int alternative(int n) { while (n) --n; return n; }\n'
                  '#else\nint no(void) { return 0; }\n#endif\n')
        self.assertEqual(list(syntax_scores(source, 'c')),
                         [(2, 'yes', 2), (4, 'alternative', 2), (6, 'no', 1)])

    def test_t843_body_bytes_and_locations_survive_header_length_boundaries(self):
        for size in [1, 2, 127, 128, 255, 4096]:
            for brackets in [('"', '"'), ('<', '>')]:
                header = brackets[0] + 'x' * size + brackets[1]
                source = '#if !__has_include(' + header + ') && FLAG\n'
                body = 'int measured(int x) { return x ? 1 : 0; }'
                source += body + '\n#else\n' + body.replace('measured', 'fallback') + '\n#endif\n'
                tree = parse(source, 'c')
                functions = [node for node in descendants(tree) if node.type == 'function_definition']
                self.assertEqual(len(functions), 2)
                for node in functions:
                    self.assertEqual(node.text, source.encode()[node.start_byte:node.end_byte])
                self.assertEqual(list(syntax_scores(source, 'c')),
                                 [(2, 'measured', 2), (4, 'fallback', 2)])

    def test_t843_malformed_headers_conditions_and_bodies_still_fail(self):
        conditions = ['__has_include("missing)', '__has_include(<missing)',
                      '__has_include("ok.h") &&', '__has_include("ok.h"',
                      '__has_include("ok.h"))', '__has_include("ok.h") garbage']
        for condition in conditions:
            with self.subTest(condition=condition), self.assertRaises(ValueError):
                list(syntax_scores('#if ' + condition + '\nint f(void){return 0;}\n#endif\n', 'c'))
        for body in ['int f( {', 'int f(void) { if (', 'int f(void) { return 1;']:
            with self.subTest(body=body), self.assertRaises(ValueError):
                list(syntax_scores('#if __has_include("ok.h")\n' + body + '\n#endif\n', 'c'))

    def test_t843_continuations_nested_guards_and_inner_function_guards(self):
        source = ('#if defined(FEATURE) && __has_include(\\\n "é.h")\n'
                  '#if __has_include /* comment */ (<nested.h>)\n'
                  'int nested(void) { return 1; }\n#endif\n#endif\n'
                  'int inner(int n) {\n#if __has_include("x.h")\n'
                  'if (n) return 1;\n#else\nwhile(n) --n;\n#endif\nreturn n;\n}\n')
        self.assertEqual(list(syntax_scores(source, 'c')),
                         [(4, 'nested', 1), (7, 'inner', 3)])

    def test_t843_comments_strings_and_ordinary_calls_are_untouched(self):
        source = ('/*\n#if __has_include("comment.h")\n*/\n'
                  'const char *s = "__has_include(\\\"literal.h\\\")";\n'
                  'int f(void) { return __has_include("argument.h"); }\n')
        self.assertEqual(parse(source, 'c').text, source.encode())
        self.assertEqual(list(syntax_scores(source, 'c')), [(5, 'f', 1)])


def descendants(node):
    yield node
    for child in node.children:
        yield from descendants(child)
