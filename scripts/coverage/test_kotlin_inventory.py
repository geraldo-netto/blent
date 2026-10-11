"""T497: compiler PSI distinguishes authored callbacks, accessors and constructors."""
from pathlib import Path
import tempfile
import os
import subprocess
import unittest
from unittest.mock import patch

import kotlin_inventory


class KotlinInventoryTest(unittest.TestCase):
    def test_t497_real_lambda_body_entries_disambiguate_shared_declaration_lines(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            source.write_text('''fun fixture() = consume {
    consume {
        println("inner")
    }
}
''')
            rows = kotlin_inventory.functions(root, [source])
            outer, inner = [row for row in rows if row.name == '<lambda>']
            self.assertEqual((outer.first, outer.body), (1, 2))
            self.assertEqual((inner.first, inner.body), (2, 3))

    def test_t497_real_compiler_inventory_and_cached_repeat_agree(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            source.write_text('''class Fixture {
    constructor(value: Int) { require(value > 0) { "positive" } }
    val value: Int get() = 7
    fun execute(block: () -> Int): Int = block()
    fun choice() = execute { value }
}
''')
            real_cache = kotlin_inventory.cache_dir()
            cache = root/'compiler'
            cache.mkdir()
            resolve = kotlin_inventory.artifact
            with patch.object(kotlin_inventory, 'cache_dir', return_value=cache), \
                    patch.object(kotlin_inventory, 'artifact', side_effect=lambda _, key, digest: resolve(real_cache, key, digest)):
                first = kotlin_inventory.functions(root, [source])
                self.assertEqual(first, kotlin_inventory.functions(root, [source]))
            self.assertEqual([row.name for row in first], ['<init>', '<lambda>', 'get:value', 'execute', 'choice', '<lambda>'])
            self.assertTrue(all(row.file == 'Fixture.kt' and row.last >= row.first > 0 for row in first))
            self.assertEqual(kotlin_inventory.functions(root, []), [])

    def test_t848_primary_constructor_collects_multiple_initializers_once(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            source.write_text('''class Fixture(val seed: Int = 1) {
    val first = seed + 1
    init { require(first > 0) }
    fun between() = first
    val second = first + 1
    init { require(second > first) }
    val callback = { second }
}
class Plain
interface Contract { fun act() }
''')
            rows = kotlin_inventory.functions(root, [source])
            constructors = [row for row in rows if row.name == '<init>']
            self.assertEqual(len(constructors), 1, 'T848 lost authored primary constructor')
            self.assertEqual((constructors[0].first, constructors[0].last), (1, 8))
            self.assertEqual([row.name for row in rows], ['<init>', 'between', '<lambda>'])

    def test_t848_object_and_companion_initializers_are_real_static_methods(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            source.write_text('''object Singleton {
    val value = System.nanoTime()
    init { require(value != 0L) }
}
class Holder {
    companion object {
        val value = System.nanoTime()
    }
}
''')
            rows = kotlin_inventory.functions(root, [source])
            self.assertEqual([(row.name, row.first, row.last) for row in rows],
                             [('<clinit>', 1, 4), ('<clinit>', 6, 8)])

    def test_t848_file_secondary_object_literal_and_enum_initialization_keep_native_boundaries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            source.write_text('''const val constant = 1
val top = System.nanoTime()
open class Base(val seed: Int)
object Derived : Base(3) { val body = seed + 1 }
class Secondary {
    val body = System.nanoTime()
    init { require(body != 0L) }
    constructor(seed: Int) { require(seed > 0) }
}
fun anonymous() = object { val body = System.nanoTime() }
enum class Kind(val code: Int) { FIRST(1), SECOND(2) }
''')
            rows = kotlin_inventory.functions(root, [source])
            self.assertEqual([(row.name, row.first, row.last) for row in rows], [
                ('<clinit>', 1, 12), ('<init>', 3, 3), ('<clinit>', 4, 4), ('<init>', 4, 4),
                ('<init>', 8, 8), ('anonymous', 10, 10), ('<init>', 10, 10),
                ('<init>', 11, 11), ('<clinit>', 11, 11)])
            constant = root/'Constants.kt'
            constant.write_text('const val fixed = 1\ninterface Empty\nclass Plain\n')
            self.assertEqual(kotlin_inventory.functions(root, [constant]), [])

    def test_t848_bounded_initializer_shapes_preserve_single_authored_method(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = []
            for count in range(1, 65):
                source = root/f'Fixture{count}.kt'
                statements = [f'    val value{n} = {n} + 1\n    init {{ require(value{n} > 0) }}' for n in range(count)]
                source.write_text('class Fixture {\n' + '\n'.join(statements) + '\n}\n')
                paths.append(source)
            rows = kotlin_inventory.functions(root, paths)
            self.assertEqual(len(rows), 64)
            self.assertTrue(all(row.name == '<init>' and row.first == 1 for row in rows))
            self.assertEqual([row.last for row in rows], [2 * count + 2 for count in range(1, 65)])

    def test_t848_init_only_bodies_and_delegated_fields_are_not_generated_scaffolding(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            source.write_text('''class InitOnly { init { System.nanoTime() } }
object InitObject { init { System.nanoTime() } }
class Delegated { val value by lazy { System.nanoTime() } }
''')
            rows = kotlin_inventory.functions(root, [source])
            self.assertEqual([(row.name, row.first) for row in rows],
                             [('<init>', 1), ('<clinit>', 2), ('<init>', 3), ('<lambda>', 3)])

    def test_t966_null_defaults_and_empty_init_are_filtered_setup_not_executable_methods(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            source.write_text('''object NullOnly { var field: Any? = null }
object Parenthesized { var field: Any? = ((null)) }
class EmptyInit { init {} }
object EmptyObject { init {} }
object Executable { var field: Any? = null; init { System.nanoTime() } }
class Parameter(val field: Any? = null)
object Delegate { val field by lazy { null } }
object Call { val field: Any? = factory() }
''')
            rows = kotlin_inventory.functions(root, [source])
            self.assertEqual([(row.name, row.first) for row in rows], [
                ('<clinit>', 5), ('<init>', 6), ('<clinit>', 7), ('<lambda>', 7), ('<clinit>', 8)])

    def test_t966_actual_compiler_default_fields_do_not_emit_authored_static_work(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            source.write_text('object NullOnly { private var owner: Any? = null; fun read() = owner }\n')
            cache = kotlin_inventory.cache_dir()
            classpath = os.pathsep.join(kotlin_inventory.artifact(cache, name, digest)
                                       for name, digest in kotlin_inventory.ARTIFACTS.items())
            subprocess.run(['java', '-cp', classpath, 'org.jetbrains.kotlin.cli.jvm.K2JVMCompiler',
                            '-no-stdlib', '-no-reflect', '-cp', classpath, '-d', str(root/'classes'), str(source)],
                           check=True, capture_output=True, timeout=30)
            native = subprocess.check_output(['javap', '-c', '-p', '-classpath', str(root/'classes'), 'NullOnly'],
                                             text=True, timeout=30)
            self.assertRegex(native, r'putstatic[^\n]*INSTANCE')
            self.assertNotRegex(native, r'putstatic[^\n]*owner')
            self.assertEqual([row.name for row in kotlin_inventory.functions(root, [source])], ['read'])

    def test_t966_bounded_parentheses_preserve_calls_and_remove_only_null_work(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'Fixture.kt'
            lines = []
            for depth in range(65):
                null = '(' * depth + 'null' + ')' * depth
                call = '(' * depth + 'factory()' + ')' * depth
                lines.extend([f'object Null{depth} {{ val value: Any? = {null} }}',
                              f'object Call{depth} {{ val value: Any? = {call} }}'])
            source.write_text('\n'.join(lines) + '\n')
            rows = kotlin_inventory.functions(root, [source])
            self.assertEqual([(row.name, row.first) for row in rows],
                             [('<clinit>', 2 * depth + 2) for depth in range(65)])

    def test_t497_compiler_result_rejects_unowned_paths_and_malformed_records(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for record in ['broken', f'{root}/../outside.kt\t1\t2\trun\t1', f'{root}/a.kt\tnan\t2\trun\t1']:
                with self.assertRaises(ValueError): kotlin_inventory.parse_result(root, record)
