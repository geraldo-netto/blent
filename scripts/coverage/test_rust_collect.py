"""T880: fresh owned targets prevent cached LLVM mappings contaminating exports."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


def run(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=60, **kwargs)


def tools():
    sysroot = Path(run('rustc', '--print', 'sysroot').stdout.strip())
    host = next(line.split(': ', 1)[1] for line in run('rustc', '-vV').stdout.splitlines()
                if line.startswith('host: '))
    return sysroot / 'lib/rustlib' / host / 'bin'


def snapshot(root):
    return {'version': 1, 'sources': {'probe.rs': hashlib.sha256((root / 'probe.rs').read_bytes()).hexdigest()},
            'functions': []}


class RustCollectTest(unittest.TestCase):
    def test_t880_native_cached_mapping_cannot_enter_fresh_collection(self):
        import rust_collect
        llvm = tools()
        with tempfile.TemporaryDirectory(prefix='blent-t880-native-') as directory:
            root = Path(directory)
            source = root / 'probe.rs'
            old = root / 'cached-target'
            old.mkdir()
            source.write_text('fn stale_only() { println!("stale"); }\nfn main() { stale_only(); }\n')
            run('rustc', '-Cinstrument-coverage', str(source), '-o', str(old / 'cached'))
            source.write_text('// current source\n// no stale function\nfn main() { println!("current"); }\n')
            inherited = {**os.environ, 'CARGO_LLVM_COV_TARGET_DIR': str(old),
                         'CARGO_TARGET_DIR': str(old), 'CARGO_LLVM_COV_BUILD_DIR': str(old),
                         'LLVM_PROFILE_FILE': str(old / 'old.profraw')}
            exporter = NativeExporter(root, old, llvm)
            output = root / 'evidence'
            rust_collect.collect(root, output, rust_collect.Selection(library=True),
                                 runner=exporter, snapshotter=snapshot, inherited=inherited)
            self.assertIn('stale_only', exporter.contaminated)
            self.assertNotIn('stale_only', (output / 'rust.lcov').read_text())
            proof = json.loads((output / 'attestation.json').read_text())
            self.assertEqual(['target/current'], list(proof['objects']))
            self.assertEqual(snapshot(root)['sources'], proof['sources'])
            self.assertEqual(proof['build_sources'], json.loads((output / 'native-sources.json').read_text()))
            self.assertEqual('true', exporter.environment['CARGO_NET_OFFLINE'])
            self.assertNotIn('LLVM_PROFILE_FILE', exporter.environment)
            self.assertIn('--offline', exporter.command)
            self.assertIn('--locked', exporter.command)
            self.assertEqual(str(output / 'target'), exporter.environment['CARGO_TARGET_DIR'])
            self.assertTrue((old / 'cached').is_file())

    def test_t880_existing_output_is_never_reused_or_deleted(self):
        import rust_collect
        with tempfile.TemporaryDirectory(prefix='blent-t880-existing-') as directory:
            root = Path(directory)
            keep = root / 'keep'
            keep.write_text('owned elsewhere')
            with self.assertRaises(FileExistsError):
                rust_collect.collect(root, root, rust_collect.Selection(), runner=lambda *a, **k: self.fail())
            self.assertEqual('owned elsewhere', keep.read_text())

    def test_t880_changed_sources_do_not_gain_attestation(self):
        import rust_collect
        with tempfile.TemporaryDirectory(prefix='blent-t880-change-') as directory:
            root = Path(directory)
            (root / 'probe.rs').write_text('fn main() {}')
            def changed(command, **kwargs):
                (root / 'probe.rs').write_text('fn main() { panic!(); }')
            with self.assertRaises(ValueError):
                rust_collect.collect(root, root / 'evidence', rust_collect.Selection(),
                                     runner=changed, snapshotter=snapshot)
            self.assertFalse((root / 'evidence/attestation.json').exists())

    def test_t880_external_native_object_link_is_rejected(self):
        import rust_collect
        with tempfile.TemporaryDirectory(prefix='blent-t880-link-') as directory:
            root = Path(directory)
            target = root / 'target'
            target.mkdir()
            (root / 'external').write_bytes(b'\x7fELFexternal')
            (target / 'linked').symlink_to(root / 'external')
            with self.assertRaises(ValueError):
                rust_collect.owned_objects(target)

    def test_t880_selection_has_no_output_or_target_passthrough(self):
        import rust_collect
        for invalid in [None, 0, '', '-p', '../other', 'a\nb', '\x00', 'a' * 257]:
            with self.assertRaises(ValueError):
                rust_collect.command(rust_collect.Selection(packages=(invalid,)), Path('evidence/rust.lcov'))
        command = rust_collect.command(rust_collect.Selection(packages=('blent-config',),
                        tests=('integration',), all_features=True, filter='bounded', library=True), Path('evidence/rust.lcov'))
        self.assertIn('--package=blent-config', command)
        self.assertIn('--test=integration', command)
        self.assertEqual(['--', 'bounded'], command[-2:])

    def test_t880_failed_or_empty_collection_cannot_claim_success(self):
        import rust_collect
        with tempfile.TemporaryDirectory(prefix='blent-t880-failure-') as directory:
            root = Path(directory)
            (root / 'probe.rs').write_text('fn main() {}')
            def failed(command, **kwargs):
                raise subprocess.CalledProcessError(1, command)
            with self.assertRaises(subprocess.CalledProcessError):
                rust_collect.collect(root, root / 'failed', rust_collect.Selection(),
                                     runner=failed, snapshotter=snapshot)
            with self.assertRaises(ValueError):
                rust_collect.collect(root, root / 'empty', rust_collect.Selection(),
                                     runner=lambda *a, **k: None, snapshotter=snapshot)
            self.assertFalse((root / 'failed/attestation.json').exists())
            self.assertFalse((root / 'empty/attestation.json').exists())

    def test_t970_explicit_features_do_not_require_all_features_or_relax_ownership(self):
        import rust_collect
        selection = rust_collect.Selection(packages=('blent',), library=True,
                                           features=('accessory-transport',))
        command = rust_collect.command(selection, Path('evidence/rust.lcov'))
        self.assertIn('--features=accessory-transport', command)
        self.assertNotIn('--all-features', command)
        self.assertIn('--offline', command)
        self.assertIn('--locked', command)
        for invalid in [None, '', '--target-dir=/tmp/old', '../other', '\x00', 'a' * 257]:
            with self.assertRaises(ValueError):
                rust_collect.command(rust_collect.Selection(features=(invalid,)), Path('evidence/rust.lcov'))
        for length in range(1, 257):
            value = 'f' * length
            self.assertIn('--features=' + value,
                rust_collect.command(rust_collect.Selection(features=(value,)), Path('evidence/rust.lcov')))


class NativeExporter:
    def __init__(self, root, old, llvm):
        self.root, self.old, self.llvm = root, old, llvm
        self.contaminated = ''
        self.environment = {}
        self.command = []

    def __call__(self, command, **kwargs):
        self.command, self.environment = command, kwargs['env']
        target = Path(self.environment['CARGO_LLVM_COV_TARGET_DIR'])
        current = target / 'current'
        run('rustc', '-Cinstrument-coverage', str(self.root / 'probe.rs'), '-o', str(current))
        raw, profile = target / 'fresh.profraw', target / 'fresh.profdata'
        run(str(current), env={**os.environ, 'LLVM_PROFILE_FILE': str(raw)})
        run(str(self.llvm / 'llvm-profdata'), 'merge', '-sparse', str(raw), '-o', str(profile))
        base = [str(self.llvm / 'llvm-cov'), 'export', '-format=lcov', '-instr-profile=' + str(profile), str(current)]
        self.contaminated = run(*base, '-object=' + str(self.old / 'cached')).stdout
        output = Path(command[command.index('--output-path') + 1])
        output.write_text(run(*base).stdout)


if __name__ == '__main__':
    unittest.main()
