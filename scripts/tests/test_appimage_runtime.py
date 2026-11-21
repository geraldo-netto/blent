"""T1622: runtime recipe, fixed inputs, cleanup grant and native lifecycle."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'packaging/appimage'))
import runtime_build


class RuntimeTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='blent-t1622-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def test_t1622_native_lifecycle_and_failures(self):
        binary = self.root / 'test'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        str(REPO / 'scripts/tests/appimage_extraction_test.c'), '-o', str(binary)], check=True)
        subprocess.run([str(binary), str(self.root)], check=True)
        self.assertEqual(list(self.root.iterdir()), [binary])

    def test_t1639_native_path_boundaries(self):
        binary = self.root / 'test'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        str(REPO / 'scripts/tests/appimage_paths_test.c'), '-o', str(binary)], check=True)
        subprocess.run([str(binary), str(self.root)], check=True)
        self.assertEqual(list(self.root.iterdir()), [binary])

    def test_t1622_depth_admission(self):
        nested = self.root
        for _ in range(63):
            nested /= 'd'
            nested.mkdir()
        (nested / 'file').touch()
        runtime_build.bundle_depth(self.root)
        (nested / 'overflow').mkdir()
        with self.assertRaisesRegex(ValueError, 'depth'):
            runtime_build.bundle_depth(self.root)

    def test_t1622_pins_before_unpacking(self):
        source = self.root / 'source'
        source.mkdir()
        target = self.root / 'target'
        inputs = {'fixture.tar': {'sha256': 'a' * 64}}
        with patch.object(runtime_build.json, 'loads', return_value=inputs), \
                patch.object(runtime_build, 'digest', return_value='b' * 64), \
                patch.object(runtime_build.tarfile, 'open') as archive:
            with self.assertRaisesRegex(ValueError, 'checksum'):
                runtime_build.unpack(source, target)
            archive.assert_not_called()
        with patch.object(runtime_build.json, 'loads', return_value=inputs), \
                patch.object(runtime_build, 'digest', return_value='a' * 64), \
                patch.object(runtime_build.tarfile, 'open') as archive:
            self.assertEqual(runtime_build.unpack(source, target), target / 'type2-runtime-20251108')
            archive.return_value.__enter__.return_value.extractall.assert_called_once_with(target, filter='data')

    def test_t1622_recipe_commands_and_failure_propagation(self):
        env = dict(os.environ, CC='/private/musl-gcc', CFLAGS='-Os')
        with patch.object(runtime_build, 'run') as run, patch.object(runtime_build.shutil, 'copyfile'):
            runtime_build.toolchain(self.root, env, 2)
            runtime_build.compression(self.root, env, 2)
            runtime_build.fuse(self.root, self.root / 'runtime', env, 2)
            source = self.root / 'squashfuse-0.5.2'
            source.mkdir(); (source / 'header.h').touch()
            runtime_build.squash(self.root, env, 2)
            runtime_build.compile_runtime(self.root, self.root / 'runtime', env)
            commands = [call.args[0] for call in run.call_args_list]
            self.assertIn(['make', '-j2'], commands)
            self.assertIn(['ninja', '-C', 'build', 'install'], commands)
            self.assertIn('extraction.c', commands[-1])
            self.assertIn('-fsanitize=bounds', commands[-1])
            self.assertIn('-fsanitize-undefined-trap-on-error', commands[-1])
        with patch.object(runtime_build.subprocess, 'run', side_effect=subprocess.CalledProcessError(7, 'fixture')):
            with self.assertRaises(subprocess.CalledProcessError):
                runtime_build.run(['fixture'], self.root, env)

    def test_t1622_mandatory_native_gate_and_source_record(self):
        steps = ['unpack', 'toolchain', 'compression', 'fuse', 'squash', 'compile_runtime', 'run', 'record']
        from contextlib import ExitStack
        with ExitStack() as stack:
            mocks = [stack.enter_context(patch.object(runtime_build, step)) for step in steps]
            mocks[1].return_value = dict(os.environ)
            binary = self.root / 'runtime'
            mocks[5].return_value = binary
            self.assertEqual(runtime_build.build(self.root, self.root / 'build'), binary)
            self.assertEqual([call.args[0][-1] for call in mocks[6].call_args_list],
                             [binary, binary.with_name('runtime-bounds')])
            self.assertTrue(str(mocks[6].call_args.args[0][1]).endswith('runtime/verify.py'))
            mocks[7].assert_called_once_with((self.root / 'build').resolve(), binary, dict(os.environ))
        for jobs in [0, 129]:
            with self.assertRaisesRegex(ValueError, 'jobs'):
                runtime_build.build(self.root, self.root / 'unused', jobs)

    def test_t1622_recipe_arguments(self):
        with patch.object(sys, 'argv', ['recipe', '--sources', 'source', '--workspace', 'work']):
            args = runtime_build.arguments()
            self.assertEqual(args.sources, Path('source'))
            self.assertEqual(args.jobs, 2)

    def test_t1622_corresponding_recipe_and_digest(self):
        binary = self.root / 'runtime'
        binary.write_bytes(b'native fixture')
        with patch.object(runtime_build.subprocess, 'check_output', return_value='fixture version\n'):
            runtime_build.record(self.root, binary, dict(os.environ))
        record = self.root / 'blent-runtime-build'
        metadata = json.loads((record / 'build.json').read_text())
        self.assertEqual(metadata['runtime_sha256'], runtime_build.digest(binary))
        self.assertEqual(len(metadata['tools']), 7)
        self.assertTrue((record / 'runtime/lifecycle.patch').is_file())
        self.assertTrue((record / 'runtime/verify.py').is_file())


if __name__ == '__main__':
    unittest.main()
