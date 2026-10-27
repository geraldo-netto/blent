"""T1281: select the actual release roots/features and normal/build closure."""
import json
import itertools
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'packaging/appimage'))
import sources
import rust_build
import rust_graph


def package(name, source='registry'):
    return dict(id=name, name=name, version='1.0.0', source=source,
                manifest_path=str(REPO / name / 'Cargo.toml'))


class RustGraphTest(unittest.TestCase):
    def setUp(self):
        self.packages = [package('host', None), package('gui', None),
                         package('normal'), package('build-helper'),
                         package('native-audio'), package('gui-only'),
                         package('dev-only'), package('dev-feature-leak')]
        self.metadata = dict(packages=self.packages, workspace_members=['host', 'gui'],
                             resolve=dict(nodes=[dict(id=p['id']) for p in self.packages]))

    def command(self, args, **kwargs):
        if args[1] == 'metadata':
            return json.dumps(self.metadata)
        if args[1] == 'tree':
            if 'host/Cargo.toml' in args:
                return 'host v1.0.0\nnormal v1.0.0\nbuild-helper v1.0.0\nnative-audio v1.0.0\nnormal v1.0.0 (*)\n'
            return 'gui v1.0.0\nnormal v1.0.0\ngui-only v1.0.0\n'
        raise AssertionError(args)

    def test_t1281_collects_only_release_closure_with_required_features(self):
        with patch.object(sources, 'run', side_effect=self.command) as run:
            with patch.object(sources, 'copy_crate', side_effect=lambda p, *args: dict(name=p['name'])):
                result = sources.rust_sources(REPO, Path('/unused'), Path('/unused'))
        self.assertEqual({r['name'] for r in result}, {'normal', 'build-helper', 'native-audio', 'gui-only'})
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(len(commands), 4)
        for command in commands:
            self.assertIn('--offline', command)
            self.assertIn('--locked', command)
            if 'host/Cargo.toml' in command:
                self.assertIn('native-audio,accessory-transport', command)
        trees = [command for command in commands if command[1] == 'tree']
        self.assertEqual(len(trees), 2)
        for command in trees:
            self.assertIn('normal,build', command)
            self.assertIn('x86_64-unknown-linux-gnu', command)
        normal = next(row for row in result if row['name'] == 'normal')
        self.assertEqual(normal['build_manifests'], ['host/Cargo.toml', 'gui/Cargo.toml'])

    def test_t1281_release_builder_uses_shared_options_and_stops_on_failure(self):
        with patch.object(rust_build.subprocess, 'run') as command:
            rust_build.build_release(REPO)
        expected = [
            ['cargo', 'build', '--release', '--locked', '--manifest-path', 'host/Cargo.toml',
             '--features', 'native-audio,accessory-transport'],
            ['cargo', 'build', '--release', '--locked', '--manifest-path', 'gui/Cargo.toml']]
        self.assertEqual([call.args[0] for call in command.call_args_list], expected)
        for call in command.call_args_list:
            self.assertEqual(call.kwargs, dict(cwd=REPO, check=True))
        failure = subprocess.CalledProcessError(1, 'cargo')
        with patch.object(rust_build.subprocess, 'run', side_effect=failure) as command:
            with self.assertRaises(subprocess.CalledProcessError):
                rust_build.build_release(REPO)
            self.assertEqual(command.call_count, 1)
        script = (REPO / 'scripts/build-release.sh').read_text()
        self.assertIn('python3 packaging/appimage/rust_build.py', script)
        self.assertNotIn('cargo build ', script)

    def test_t1281_invalid_ambiguous_or_unmatched_tree_rows_fail_closed(self):
        index = rust_graph.package_index(self.metadata)
        invalid = ['', 'normal', 'normal 1.0.0', ' normal v1.0.0', 'normal v1.0.0 garbage',
                   'absent v1.0.0', 'normal v2.0.0', '\x1b[0mnormal v1.0.0']
        for line in invalid:
            with self.subTest(line=line), self.assertRaises(ValueError):
                rust_graph.tree_package(line, index)
        index['normal', '1.0.0'].append(dict(index['normal', '1.0.0'][0], id='other-registry'))
        with self.assertRaisesRegex(ValueError, 'ambiguous'):
            rust_graph.tree_package('normal v1.0.0', index)

    def test_t1281_duplicate_metadata_and_missing_root_are_rejected(self):
        self.metadata['packages'].append(self.packages[-1])
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            rust_graph.package_index(self.metadata)
        self.metadata['packages'].pop()
        with self.assertRaisesRegex(ValueError, 'unique workspace'):
            rust_graph.root_package(self.metadata, REPO, 'missing/Cargo.toml')
        self.metadata['workspace_members'].remove('host')
        with self.assertRaisesRegex(ValueError, 'unique workspace'):
            rust_graph.root_package(self.metadata, REPO, 'host/Cargo.toml')

    def test_t1281_empty_wrong_root_and_external_path_graphs_are_rejected(self):
        for tree in ['', 'normal v1.0.0\n', 'gui v1.0.0\n']:
            with self.subTest(tree=tree), self.assertRaisesRegex(ValueError, 'release root'):
                rust_graph.select_tree(self.metadata, tree, self.packages[0])
        self.packages[2]['source'] = None
        with self.assertRaisesRegex(ValueError, 'external path dependency'):
            rust_graph.select_tree(self.metadata, 'host v1.0.0\nnormal v1.0.0\n', self.packages[0])

    def test_t1281_metadata_change_between_builds_is_not_silently_merged(self):
        result = {}
        rust_graph.merge_packages(result, [self.packages[2]], 'host/Cargo.toml')
        changed = dict(self.packages[2], source='different-registry')
        with self.assertRaisesRegex(ValueError, 'changed between release queries'):
            rust_graph.merge_packages(result, [changed], 'gui/Cargo.toml')

    def test_t1281_deduplicates_shared_and_repeated_edges_with_bounded_permutations(self):
        for names in itertools.permutations(['normal', 'build-helper', 'gui-only']):
            tree = 'host v1.0.0 (/path with spaces)\n'
            tree += ''.join(f'{name} v1.0.0\n{name} v1.0.0 (*)\n' for name in names)
            result = list(rust_graph.select_tree(self.metadata, tree, self.packages[0]))
            self.assertEqual({p['name'] for p in result}, set(names))
            self.assertEqual(len(result), 3)


class CargoResolverTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='blent-cargo-graph-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def crate(self, name, manifest=''):
        path = self.root / name
        (path / 'src').mkdir(parents=True)
        (path / 'src/lib.rs').write_text('')
        (path / 'Cargo.toml').write_text(f'[package]\nname="{name}"\nversion="1.0.0"\nedition="2021"\n' + manifest)

    def run_cargo(self, args, **kwargs):
        return subprocess.run(args, check=True, capture_output=True, text=True, **kwargs).stdout

    def test_t1281_real_cargo_excludes_dev_feature_unification(self):
        (self.root / 'Cargo.toml').write_text('[workspace]\nresolver="2"\nmembers=["host","common","audio","build-helper","dev-only","optional"]\n')
        self.crate('host', '''[features]
native-audio=["dep:audio"]
accessory-transport=[]
[dependencies]
common={path="../common"}
audio={path="../audio",optional=true}
[build-dependencies]
build-helper={path="../build-helper"}
[dev-dependencies]
dev-only={path="../dev-only"}
''')
        self.crate('common', '''[features]
extra=["dep:optional"]
[dependencies]
optional={path="../optional",optional=true}
''')
        self.crate('dev-only', '[dependencies]\ncommon={path="../common",features=["extra"]}\n')
        for name in ['audio', 'build-helper', 'optional']:
            self.crate(name)
        self.run_cargo(['cargo', 'generate-lockfile', '--offline'], cwd=self.root)
        metadata, tree = rust_graph.query(self.root, rust_build.BUILD_UNITS[0], self.run_cargo)
        index = rust_graph.package_index(metadata)
        names = {rust_graph.tree_package(line, index)['name'] for line in tree.splitlines()}
        self.assertEqual(names, {'host', 'common', 'audio', 'build-helper'})
        common_id = next(p['id'] for p in metadata['packages'] if p['name'] == 'common')
        common = next(node for node in metadata['resolve']['nodes'] if node['id'] == common_id)
        self.assertIn('extra', common['features'])
