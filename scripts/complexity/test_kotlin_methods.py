"""T965: authored accessors, construction and callbacks belong in the audit."""
from pathlib import Path
import tempfile
import unittest

from kotlin import kotlin_scores


def scores(source):
    with tempfile.TemporaryDirectory(prefix='blent-t965-') as directory:
        path = Path(directory) / 'fixture.kt'
        path.write_text(source)
        return [(name, score) for _, _, name, score in kotlin_scores([path])]


class KotlinMethodsTest(unittest.TestCase):
    def test_t965_primary_initialization_excludes_unrelated_member_methods(self):
        result = scores('class C(val initial: Int = if (flag) 1 else 2) {\n'
                        'val value = if (initial > 0) initial else 0\n'
                        'init { if (value > 0) work() }\n'
                        'fun named() { if (flag) work() }\n}')
        self.assertEqual([('<init>', 4), ('named', 2)], result)

    def test_t965_secondary_constructor_and_authored_accessors(self):
        result = scores('class C { constructor() { if (flag) work() }\n'
                        'var value = 0\n'
                        'get() { if (flag) return field; return 0 }\n'
                        'set(v) { if (v > 0) field = v } }')
        self.assertEqual([('<init>', 2), ('get:value', 2), ('set:value', 2)], result)

    def test_t965_callbacks_have_rows_and_existing_named_counts_stay(self):
        result = scores('val callback = { if (flag && other) work() }\n'
                        'fun owner() { consume { if (flag) work() } }')
        self.assertEqual([('<clinit>', 3), ('<lambda>', 3), ('owner', 2), ('<lambda>', 2)], result)

    def test_t965_nonexecutable_setup_and_abstract_scaffolding_stay_absent(self):
        self.assertEqual([], scores('object Singleton { private var owner: Any? = null }\n'
                                    'class Empty { init {} }\ninterface Contract { fun unimplemented() }'))

    def test_t965_named_object_supercall_has_separate_initialization_owners(self):
        result = scores('object Single : Base(if (flag) 1 else 2) { val x = if (other) 3 else 4 }')
        self.assertEqual([('<clinit>', 2), ('<init>', 2)], result)

    def test_t965_nested_object_methods_do_not_inflate_outer_constructor(self):
        result = scores('class C { val callback = object { init { if (flag) work() }\n'
                        'fun run() { if (other) work() } } }')
        self.assertEqual([('<init>', 1), ('<init>', 2), ('run', 2)], result)

    def test_t965_secondary_delegation_counts_shared_initialization_once(self):
        result = scores('class C { val x = if (flag) 1 else 2\n'
                        'constructor(seed: Int) { if (seed > 0) work() }\n'
                        'constructor() : this(1) { if (other) work() } }')
        self.assertEqual([('<init>', 3), ('<init>', 2)], result)

    def test_t965_enum_arguments_belong_only_to_static_initialization(self):
        result = scores('enum class Choice(val x: Int) {\n'
                        'A(if (flag) 1 else 2) { init { if (other) work() } }, B(0) }')
        self.assertEqual([('<init>', 1), ('<clinit>', 2), ('<init>', 2)], result)

    def test_t965_bounded_method_kinds_retain_nine_ten_boundary(self):
        templates = ['class C { val x: Int get() { BODY return 0 } }',
                     'class C { constructor() { BODY } }',
                     'class C() { init { BODY } }',
                     'val callback = { BODY }']
        with tempfile.TemporaryDirectory(prefix='blent-t965-boundaries-') as directory:
            paths = []
            for kind, template in enumerate(templates):
                for count in range(1, 14):
                    path = Path(directory) / f'kind{kind}count{count}.kt'
                    path.write_text(template.replace('BODY', 'if (flag) work(); ' * (count - 1)))
                    paths.append(path)
            rows = kotlin_scores(paths)
            actual = {(Path(path).name, name): score for path, _, name, score in rows}
            for kind, name in enumerate(['get:x', '<init>', '<init>', '<lambda>']):
                for count in range(1, 14):
                    self.assertEqual(count, actual[(f'kind{kind}count{count}.kt', name)])


if __name__ == '__main__':
    unittest.main()
