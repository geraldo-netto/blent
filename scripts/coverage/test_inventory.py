"""T497: fixtures cannot hide production code or inflate enclosing functions."""
from pathlib import Path
import tempfile
import unittest

from inventory import coverage_source, fixture, rust_test_files, syntax_functions
from model import result


class InventoryTest(unittest.TestCase):
    def test_t1221_async_closures_preserve_function_and_test_boundaries(self):
        source = ('// café 🦀\n#[allow(dead_code)]\npub async fn outer() {\n'
                  '    fn inner() { work(); }\n'
                  '    let callback = async move || { work().await; };\n}\n'
                  '#[cfg(test)] mod tests { async fn fixture() { let c = async || {}; } }\n')
        functions = list(syntax_functions(Path('lib.rs'), source, 'rust'))
        self.assertEqual([(fn.name, fn.first, fn.last, fn.nested) for fn in functions],
                         [('outer', 3, 6, ((4, 4),)), ('inner', 4, 4, ())])

    def test_t831_exempt_artifacts_cannot_break_production_inventory(self):
        import subprocess
        from report import snapshot
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            for name, source in [('host/src/main.rs', 'fn main() {}'),
                                 ('docs/reviews/candidate.c', 'invalid {{{'),
                                 ('scripts/benchmarks/probe.rs', 'invalid {{{')]:
                path = root/name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)
            manifest = snapshot(root)
            self.assertEqual(list(manifest['sources']), ['host/src/main.rs'])
            self.assertEqual([fn['name'] for fn in manifest['functions']], ['main'])
            (root/'host/src/main.rs').write_text('invalid {{{')
            with self.assertRaisesRegex(ValueError, 'parse error'):
                snapshot(root)

    def test_t972_native_inputs_preserve_exempt_benchmark_hashes_without_threshold_functions(self):
        import json
        import subprocess
        from model import fingerprint
        from platform_scope import attest_native
        from report import snapshot, validate_snapshot
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            production = root/'host/src/main.rs'
            benchmark = root/'scripts/benchmarks/probe.rs'
            for path, text in [(production, 'fn main() {}'), (benchmark, 'fn benchmark() {}')]:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text)
            manifest = snapshot(root)
            self.assertEqual(list(manifest['sources']), ['host/src/main.rs'])
            self.assertEqual([fn['name'] for fn in manifest['functions']], ['main'])
            inputs = manifest.get('build_sources', manifest['sources'])
            stamp = root/'native-sources.json'
            stamp.write_text(json.dumps(inputs))
            counters = {str(path.relative_to(root)): {1: 1} for path in [production, benchmark]}
            attest_native(root, [stamp], counters, ['linux'])
            self.assertEqual(inputs['scripts/benchmarks/probe.rs'], fingerprint(benchmark))
            validate_snapshot(root, manifest)
            benchmark.write_text('fn changed() {}')
            with self.assertRaises(ValueError): validate_snapshot(root, manifest)
            with self.assertRaises(ValueError): attest_native(root, [stamp], counters, ['linux'])


    def test_t972_missing_added_changed_or_invalid_build_inputs_fail_closed(self):
        import copy
        import subprocess
        from report import snapshot, validate_snapshot
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            source = root/'scripts/benchmarks/probe.rs'
            source.parent.mkdir(parents=True)
            source.write_text('invalid Rust stays fingerprinted, never parsed')
            manifest = snapshot(root)
            self.assertEqual(manifest['functions'], [])
            self.assertEqual(manifest['sources'], {})
            validate_snapshot(root, manifest)
            for invalid in [None, [], {}, {'../escape.rs': '0'*64}, {str(source.relative_to(root)): '0'*64}]:
                damaged = copy.deepcopy(manifest)
                damaged['build_sources'] = invalid
                with self.assertRaises(ValueError): validate_snapshot(root, damaged)
            source.unlink()
            with self.assertRaises(ValueError): validate_snapshot(root, manifest)
            source.write_text('invalid Rust stays fingerprinted, never parsed')
            extra = root/'scripts/benchmarks/extra.rs'
            extra.write_text('fn added() {}')
            with self.assertRaises(ValueError): validate_snapshot(root, manifest)


    def test_t846_native_accessory_functions_remain_in_production_scope(self):
        self.assertTrue(coverage_source(Path('host/accessory/stream.c')))
        self.assertTrue(coverage_source(Path('host/accessory/accessory.h')))

    def test_t766_optional_gpu_functions_remain_in_production_scope(self):
        self.assertTrue(coverage_source(Path('host/gpu/capture.c')))
        self.assertTrue(coverage_source(Path('host/gpu/options.h')))

    def test_t497_combined_inventory_keeps_inline_capture_header_functions(self):
        import subprocess
        from report import snapshot
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            header = root/'host/evdi/geometry.h'
            header.parent.mkdir(parents=True)
            header.write_text('static inline int clipped(int value) { return value < 0 ? 0 : value; }\n')
            manifest = snapshot(root)
            self.assertIn('host/evdi/geometry.h', manifest['sources'])
            self.assertEqual([function['name'] for function in manifest['functions']], ['clipped'])


    def test_t497_application_scope_excludes_benchmarks_in_every_language(self):
        for name in ['host/src/main.rs', 'common/src/lib.rs', 'host/evdi/evdi_helper.c', 'android/app/src/main/java/com/blent/MainActivity.kt',
                     'scripts/install.sh', 'scripts/setup-evdi.sh', 'scripts/gpu-boot-order.sh', 'scripts/ci/verify-portability.py',
                     'packaging/appimage/build.py', 'packaging/arch/PKGBUILD', 'packaging/appimage/AppRun']:
            self.assertTrue(coverage_source(Path(name)), name)
        for name in ['common/examples/encoder-options.rs', 'host/benches/queue.rs', 'scripts/benchmarks/main.rs', 'scripts/benchmarks/helper.c',
                     'scripts/benchmarks/app/Main.kt', 'scripts/coverage/KotlinFunctions.kt', 'scripts/fake-tablet.py',
                     'scripts/ci/gui-smoke.sh', 'docs/benchmarks/fixture.c']:
            self.assertFalse(coverage_source(Path(name)), name)

    def test_t497_rust_cfg_test_modules_are_excluded_but_not_production(self):
        source = '''fn production() { }
#[cfg(test)] mod tests { fn fixture() {} }
#[cfg(not(test))] fn normal_only() {}
#[cfg(any(test, feature = "runtime"))] fn shared() {}
'''
        functions = list(syntax_functions(Path('lib.rs'), source, 'rust'))
        self.assertEqual([fn.name for fn in functions], ['production', 'normal_only', 'shared'])
        self.assertTrue(fixture(Path('host/tests/helper.c')))
        self.assertFalse(fixture(Path('host/src/production.rs')))

    def test_t497_external_test_modules_preserve_shared_production_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'lib.rs').write_text('''#[cfg(test)] #[path="fixture.rs"] mod a;
#[path="fixture.rs"] mod b;
#[cfg(test)] #[path="profile.rs"] mod c;
''')
            self.assertEqual(rust_test_files(root, [Path('lib.rs')]), {root/'profile.rs'})

    def test_t497_nested_functions_cannot_inflate_the_parent(self):
        source = 'fn outer() {\n fn inner() {\n  todo!();\n }\n inner();\n}\n'
        functions = list(syntax_functions(Path('a.rs'), source, 'rust'))
        self.assertEqual(functions[0].nested, ((2, 4),))
        data = {'a.rs': {1: 1, 2: 0, 3: 0, 4: 0, 5: 1, 6: 1}}
        self.assertEqual(result(functions[0], data)['percent'], 100)
        self.assertEqual(result(functions[1], data)['percent'], 0)

    def test_t497_implicit_test_module_paths_propagate_without_hiding_shared_code(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'lib.rs').write_text('#[cfg(test)] mod contracts;\nmod shared;\n')
            (root/'contracts.rs').write_text('#[path="shared.rs"] mod shared;\nmod nested;\n')
            (root/'shared.rs').write_text('fn production() {}')
            (root/'contracts').mkdir()
            (root/'contracts/nested.rs').write_text('fn fixture() {}')
            files = [path.relative_to(root) for path in root.rglob('*.rs')]
            self.assertEqual(rust_test_files(root, files), {root/'contracts.rs', root/'contracts/nested.rs'})


if __name__ == '__main__':
    unittest.main()
