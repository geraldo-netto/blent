"""T1622: build and qualify the patched runtime from checksum-pinned sources."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile

from tools import digest


def unpack(sources, workspace):
    inputs = json.loads(Path(__file__).with_name('runtime-inputs.json').read_text())
    for name, entry in inputs.items():
        archive = sources / name
        if digest(archive) != entry['sha256']:
            raise ValueError(f'runtime source checksum mismatch: {name}')
        with tarfile.open(archive) as stream:
            stream.extractall(workspace, filter='data')
    return workspace / 'type2-runtime-20251108'


def run(command, directory, env):
    print('runtime build:', *command, flush=True)
    subprocess.run([str(value) for value in command], cwd=directory, env=env, check=True)


def make_install(source, env, jobs):
    run(['make', f'-j{jobs}'], source, env)
    run(['make', 'install'], source, env)


def toolchain(workspace, env, jobs):
    prefix = workspace / 'sysroot'
    musl = workspace / 'musl-1.2.5'
    run(['./configure', f'--prefix={prefix}', '--disable-shared'], musl, env)
    make_install(musl, env, jobs)
    return dict(env, CC=str(prefix / 'bin/musl-gcc'),
                CFLAGS='-Os -ffunction-sections -fdata-sections',
                PKG_CONFIG_LIBDIR=str(prefix / 'lib/pkgconfig'))


def compression(workspace, env, jobs):
    prefix = workspace / 'sysroot'
    zlib = workspace / 'zlib-1.3.1'
    run(['./configure', f'--prefix={prefix}', '--static'], zlib, env)
    make_install(zlib, env, jobs)
    zstd = workspace / 'zstd-1.5.6'
    run(['make', f'-j{jobs}', '-C', 'lib', 'libzstd.a', f"CC={env['CC']}",
         f"CFLAGS={env['CFLAGS']}"], zstd, env)
    run(['make', '-C', 'lib', 'install-static', 'install-includes', 'install-pc',
         f'PREFIX={prefix}'], zstd, env)


def fuse(workspace, runtime, env, jobs):
    prefix = workspace / 'sysroot'
    source = workspace / 'fuse-3.15.0'
    run(['patch', '-p1', '-i', runtime / 'patches/libfuse/mount.c.diff'], source, env)
    run(['meson', 'setup', 'build', f'--prefix={prefix}', '--libdir=lib',
         '--default-library=static', '-Dutils=false', '-Dexamples=false',
         '-Dtests=false', '-Duseroot=false', '-Dinitscriptdir='], source, dict(env, LDFLAGS='-static'))
    run(['ninja', '-C', 'build', f'-j{jobs}'], source, env)
    run(['ninja', '-C', 'build', 'install'], source, env)


def squash(workspace, env, jobs):
    prefix = workspace / 'sysroot'
    source = workspace / 'squashfuse-0.5.2'
    run(['./autogen.sh'], source, env)
    run(['./configure', f'--prefix={prefix}', '--disable-shared', '--enable-static', '--disable-demo'],
        source, dict(env, CPPFLAGS=f'-I{prefix}/include', LDFLAGS=f'-static -L{prefix}/lib'))
    make_install(source, env, jobs)
    headers = prefix / 'include/squashfuse'
    headers.mkdir(parents=True, exist_ok=True)
    for header in source.glob('*.h'):
        shutil.copyfile(header, headers / header.name)


def compile_runtime(workspace, runtime, env):
    adapter = Path(__file__).with_name('runtime')
    for name in ('lifecycle.patch', 'arguments.patch'):
        run(['patch', '-p1', '-i', adapter / name], runtime, env)
    source = runtime / 'src/runtime'
    for name in ('extraction.c', 'extraction.h'):
        shutil.copyfile(adapter / name, source / name)
    compile_variant(workspace / 'sysroot', source, env, 'runtime', [])
    compile_variant(workspace / 'sysroot', source, env, 'runtime-bounds',
                    ['-fsanitize=bounds', '-fsanitize-undefined-trap-on-error'])
    return source / 'runtime'


def compile_variant(prefix, source, env, output, instrumentation):
    flags = ['-std=gnu99', '-Os', '-D_FILE_OFFSET_BITS=64', '-DGIT_COMMIT="20251108-blent"',
             '-T', 'data_sections.ld', '-ffunction-sections', '-fdata-sections', '-Wl,--gc-sections',
             '-static', '-Wall', '-static-pie', f'-I{prefix}/include/squashfuse',
             f'-I{prefix}/include/fuse3', f'-L{prefix}/lib']
    run([env['CC'], *flags, *instrumentation, 'runtime.c', 'extraction.c', '-lsquashfuse', '-lsquashfuse_ll',
         '-lzstd', '-lz', '-lfuse3', '-o', output], source, env)


def bundle_depth(appdir):
    # musl nftw's descriptor grant also bounds traversal depth; reject before packaging.
    for directory, children, _ in os.walk(appdir, followlinks=False):
        depth = len(Path(directory).relative_to(appdir).parts)
        if depth >= 63 and children:
            raise ValueError('AppImage exceeds the runtime cleanup depth grant (63)')


def record(sources, runtime, env):
    package = Path(__file__).parent
    destination = sources / 'blent-runtime-build'
    destination.mkdir()
    for name in ('runtime_build.py', 'runtime-inputs.json', 'tools.py'):
        shutil.copyfile(package / name, destination / name)
    shutil.copytree(package / 'runtime', destination / 'runtime')
    versions = {}
    for tool in ('cc', 'make', 'meson', 'ninja', 'autoconf', 'automake', 'libtoolize'):
        versions[tool] = subprocess.check_output([tool, '--version'], env=env, text=True).splitlines()[0]
    (destination / 'build.json').write_text(json.dumps(dict(runtime_sha256=digest(runtime), tools=versions), indent=2) + '\n')


def build(sources, workspace, jobs=2):
    if not 1 <= jobs <= 128:
        raise ValueError('runtime build jobs must be between 1 and 128')
    workspace = workspace.resolve()
    workspace.mkdir()
    runtime = unpack(sources, workspace)
    env = toolchain(workspace, dict(os.environ), jobs)
    compression(workspace, env, jobs)
    fuse(workspace, runtime, env, jobs)
    squash(workspace, env, jobs)
    binary = compile_runtime(workspace, runtime, env)
    fixture = Path(__file__).with_name('runtime') / 'verify.py'
    for candidate in (binary, binary.with_name('runtime-bounds')):
        run(['python3', fixture, candidate], workspace, env)
    record(workspace, binary, env)
    return binary


def arguments():
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sources', required=True, type=Path)
    parser.add_argument('--workspace', required=True, type=Path)
    parser.add_argument('--jobs', type=int, choices=range(1, 129), default=2)
    return parser.parse_args()


if __name__ == '__main__':
    args = arguments()
    print(build(args.sources.resolve(), args.workspace, args.jobs))
