"""Join Cargo's normal/build trees to exact package metadata (T1281)."""
import json
from pathlib import Path
import re

from rust_build import BUILD_UNITS, TARGET, build_options


def query(repo, unit, run):
    options = ['--offline', '--locked', *build_options(unit)]
    metadata = json.loads(run(['cargo', 'metadata', '--format-version=1',
                              f'--filter-platform={TARGET}', *options], cwd=repo))
    tree = run(['cargo', 'tree', '--target', TARGET, '--edges', 'normal,build',
                '--prefix', 'none', '--format', '{p}', '--color', 'never', *options], cwd=repo)
    return metadata, tree


def package_index(metadata):
    index = {}
    identities = set()
    for package in metadata['packages']:
        if package['id'] in identities:
            raise ValueError('duplicate Cargo package identity')
        identities.add(package['id'])
        key = package['name'], package['version']
        index.setdefault(key, []).append(package)
    return index


def tree_package(line, index):
    match = re.fullmatch(r'([A-Za-z0-9_-]+) v([^\s]+)(?: \(.+\))?', line.removesuffix(' (*)'))
    if match is None:
        raise ValueError(f'invalid Cargo dependency tree row: {line}')
    candidates = index.get((match[1], match[2]), [])
    if len(candidates) != 1:
        raise ValueError(f'unmatched or ambiguous Cargo dependency identity: {line}')
    return candidates[0]


def root_package(metadata, repo, manifest):
    expected = (repo / manifest).resolve()
    roots = [p for p in metadata['packages'] if Path(p['manifest_path']).resolve() == expected]
    if len(roots) != 1 or roots[0]['id'] not in metadata['workspace_members']:
        raise ValueError('release root is not a unique workspace package')
    return roots[0]


def select_tree(metadata, tree, root):
    index = package_index(metadata)
    selected = [tree_package(line, index) for line in tree.splitlines()]
    if not selected or selected[0]['id'] != root['id']:
        raise ValueError('Cargo dependency tree lacks the selected release root')
    packages = {}
    for package in selected:
        if package['source'] is not None:
            packages[package['id']] = package
        elif package['id'] not in metadata['workspace_members']:
            raise ValueError('external path dependency needs explicit source provenance')
    return packages.values()


def merge_packages(result, packages, manifest):
    for package in packages:
        entry = result.setdefault(package['id'], dict(package=package, build_manifests=[]))
        if entry['package'] != package:
            raise ValueError('Cargo package metadata changed between release queries')
        entry['build_manifests'].append(manifest)


def release_packages(repo, run):
    result = {}
    for unit in BUILD_UNITS:
        metadata, tree = query(repo, unit, run)
        root = root_package(metadata, repo, unit[0])
        merge_packages(result, select_tree(metadata, tree, root), unit[0])
    return [result[key] for key in sorted(result)]
