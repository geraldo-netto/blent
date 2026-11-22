"""T1611: the production EVDI recipe preserves strict flags and all modules."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
MODULES = {
    'evdi_helper.c', 'conversion.c', 'frame_exchange.c', 'fifo_writer.c',
    'capture.c', 'capture_activity.c', 'capture_control.c', 'cursor_overlay.c',
    'raw_ring.c', 'writer.c', 'idle_control.c',
}


def copy_helper(root):
    shutil.copytree(REPO / 'host/evdi', root / 'host/evdi',
                    ignore=shutil.ignore_patterns('evdi_helper', '*.gc*'))


def compiler_spy(root):
    compiler = root / 'compiler'
    compiler.write_text('#!/usr/bin/python3\nimport json, os, sys\n'
                        'with open(os.environ["T1611_ARGS"], "w") as out:\n'
                        ' json.dump(sys.argv[1:], out)\n')
    compiler.chmod(0o700)
    return compiler


class EvdiBuildTest(unittest.TestCase):
    def test_t1611_local_build_uses_strict_shared_recipe(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1611-') as directory:
            root = Path(directory)
            shutil.copy(REPO / 'Makefile', root / 'Makefile')
            copy_helper(root)
            compiler = compiler_spy(root)
            env = dict(os.environ, T1611_ARGS=str(root / 'arguments'))
            result = subprocess.run(['make', 'build-helper', f'CC={compiler}'],
                                    cwd=root, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            args = json.loads((root / 'arguments').read_text())
            for flag in ['-std=c11', '-Wall', '-Wextra', '-Werror', '-O3']:
                self.assertIn(flag, args, f'T1611: missing required compiler flag {flag}')
            sources = [Path(arg).name for arg in args if arg.endswith('.c')]
            self.assertEqual(set(sources), MODULES)
            self.assertEqual(len(sources), len(MODULES))
            self.assertIn('-levdi', args)
            self.assertIn('-lpthread', args)
            self.assertIn('-Wl,-rpath,$ORIGIN', args)

    def test_t1611_shared_recipe_preserves_literal_output_and_library_paths(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1611-paths-') as directory:
            root = Path(directory)
            copy_helper(root)
            compiler = compiler_spy(root)
            output = root / "output 'space' $(touch INJECTED)`touch INJECTED`/helper"
            library = root / "library 'space' $(touch INJECTED)`touch INJECTED`"
            env = dict(os.environ, T1611_ARGS=str(root / 'arguments'),
                       OUTPUT=str(output), EVDI_LIBRARY_DIR=str(library))
            result = subprocess.run(['make', '-C', 'host/evdi', f'CC={compiler}'],
                                    cwd=root, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            args = json.loads((root / 'arguments').read_text())
            self.assertEqual(args[args.index('-o') + 1], str(output))
            self.assertIn('-L' + str(library), args)
            self.assertTrue(output.parent.is_dir())
            self.assertEqual(list(root.rglob('INJECTED')), [])
