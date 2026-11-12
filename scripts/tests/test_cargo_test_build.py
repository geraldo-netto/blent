"""T1355: preserve selected build inputs without adding test execution targets."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]/'ci'))
from cargo_test_build import binary_build


class BinaryBuildTest(unittest.TestCase):
    def test_t1355_features_profile_paths_and_package_selection_survive(self):
        original = ['cargo', 'test', '--bins', '--lib', '-p', 'blent', '--features=native-encoder',
                    '--no-default-features', '--profile', 'test', '--target-dir', 'literal space',
                    '--locked', '--offline', '--no-fail-fast', 'worker', '--', '--exact']
        self.assertEqual(binary_build(original), ['cargo', 'build', '--bins', '-p', 'blent',
            '--features=native-encoder', '--no-default-features', '--profile', 'test',
            '--target-dir', 'literal space', '--locked', '--offline'])
        self.assertEqual(original[-3:], ['worker', '--', '--exact'])

    def test_t1355_other_commands_and_harness_arguments_do_not_request_builds(self):
        for command in [['cargo', 'build', '--bins'], ['cargo', 'test', '--lib'],
                        ['cargo', 'test', '--', '--bins']]:
            self.assertIsNone(binary_build(command))
        self.assertEqual(binary_build(['cargo', 'test', '--bins', '--test', 'fixture',
                                      '--example=probe', '--release']),
                         ['cargo', 'build', '--bins', '--release'])

    def test_t1355_unknown_and_incomplete_options_reject_before_build(self):
        for option in ['--unknown', '--jobs', '--profile', '--test']:
            with self.subTest(option=option), self.assertRaises(ValueError):
                binary_build(['cargo', 'test', '--bins', option])
