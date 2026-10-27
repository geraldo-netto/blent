"""Pinned, offline notice overlays; Cargo's source cache is read-only (T1280)."""
from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
import shutil
import tempfile

from crate_notices import copy_notice, declared_notice, regular_reader

CATALOG = Path(__file__).with_name('supplemental-notices') / 'manifest.json'
OVERLAY = '.blent-upstream-notices'


def read_json(path):
    with regular_reader(path) as stream:
        content = stream.read(1024 * 1024 + 1)
    if len(content) > 1024 * 1024:
        raise ValueError('notice metadata exceeds 1 MiB')
    return json.loads(content)


def selected_pin(package):
    catalog = read_json(CATALOG)
    if catalog['schema'] != 1:
        raise ValueError('unsupported supplemental notice schema')
    key = f"{package['name']}-{package['version']}"
    pin = catalog['crates'].get(key)
    if pin is None and any(p['name'] == package['name'] for p in catalog['crates'].values()):
        raise ValueError('crate version requires an updated supplemental notice pin')
    return pin


def verify_identity(package, source, pin):
    for field in ['name', 'version', 'source', 'license', 'license_file']:
        if package[field] != pin[field]:
            raise ValueError(f'supplemental notice crate identity mismatch: {field}')
    with regular_reader(source / 'Cargo.toml') as stream:
        checksum = hashlib.file_digest(stream, 'sha256').hexdigest()
    if checksum != pin['manifest_sha256']:
        raise ValueError('supplemental notice crate manifest mismatch')
    vcs = read_json(source / '.cargo_vcs_info.json')
    if vcs != pin['vcs_info'] or vcs['git']['sha1'] != pin['revision']:
        raise ValueError('supplemental notice upstream revision mismatch')


def install_notice(source, pin, notice):
    relative = declared_notice(CATALOG.parent, notice['file'])
    if relative is None or notice['revision'] != pin['revision']:
        raise ValueError('invalid supplemental notice provenance')
    path = Path(OVERLAY) / relative
    checksum = copy_notice(CATALOG.parent / relative, source / path)
    if checksum != notice['sha256']:
        raise ValueError('supplemental notice checksum mismatch')
    provenance = dict(notice, crate_sha256=pin['crate_sha256'], vcs_info=pin['vcs_info'])
    return path.as_posix(), provenance


def install_overlay(source, pin):
    if not pin['notices']:
        raise ValueError('supplemental notice pin contains no notices')
    if (source / OVERLAY).exists() or (source / OVERLAY).is_symlink():
        raise ValueError('crate occupies reserved supplemental notice path')
    return dict(install_notice(source, pin, notice) for notice in pin['notices'])


@contextmanager
def prepared_source(package):
    source = Path(package['manifest_path']).parent
    pin = selected_pin(package)
    if pin is None:
        yield source, {}
        return
    verify_identity(package, source, pin)
    with tempfile.TemporaryDirectory(prefix='blent-crate-notices-') as workspace:
        staged = Path(workspace) / 'source'
        shutil.copytree(source, staged, symlinks=True)
        # Recheck the copied identity before admitting supplements.
        verify_identity(package, staged, pin)
        provenance = install_overlay(staged, pin)
        yield staged, provenance
