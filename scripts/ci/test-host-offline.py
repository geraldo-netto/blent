#!/usr/bin/env python3
"""Run the complete host suite with a cached SDK ABI and real host fixture tools."""
import argparse
import os
from pathlib import Path
import shlex
import shutil
import tempfile


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sdk', required=True, type=Path, help='cached SDK files directory')
    parser.add_argument('--codecs', required=True, type=Path, help='matching cached codecs-extra files directory')
    parser.add_argument('--target-dir', required=True, type=Path, help='owned build/cache directory')
    parser.add_argument('--libclang', required=True, type=Path, help='installed libclang directory')
    parser.add_argument('cargo_args', nargs=argparse.REMAINDER, help='optional cargo command, always offline/locked')
    return parser.parse_args()


def dependencies(sdk, codecs, libclang):
    for name in ('cargo', 'cc', 'pkg-config', 'dbus-run-session', 'xvfb-run', 'Xvfb', 'xdpyinfo', 'pipewire', 'pw-cat', 'pw-dump', 'pw-cli', 'pw-link', 'unshare'):
        if shutil.which(name) is None:
            raise ValueError(f'missing installed fixture tool: {name}')
    for path in (sdk/'include/pipewire-0.3/pipewire/pipewire.h',
                 sdk/'lib/x86_64-linux-gnu/ld-linux-x86-64.so.2',
                 codecs/'lib/libavcodec.so.61', libclang/'libclang.so.1'):
        if not path.is_file():
            raise ValueError(f'missing native prerequisite: {path}')


def matching_codecs(sdk, codecs):
    library = sdk/'lib/x86_64-linux-gnu'
    for name in ('avcodec', 'avutil', 'avdevice', 'avformat', 'avfilter', 'swresample', 'swscale'):
        original = (library/f'lib{name}.so').resolve()
        selected = codecs/'lib'/original.name
        if not original.is_file() or not selected.is_file():
            raise ValueError(f'matching cached codec library required: {selected}')


def sysroot(target, sdk):
    root = target/'sdk-sysroot'
    root.mkdir(parents=True, exist_ok=True)
    prefix = root/'usr'
    if prefix.is_symlink() and prefix.resolve() == sdk:
        return root
    if prefix.exists() or prefix.is_symlink():
        raise ValueError(f'foreign sysroot prefix retained: {prefix}')
    prefix.symlink_to(sdk, target_is_directory=True)
    return root


def environment(sdk, codecs, target, libclang, inherited):
    result = dict(inherited)
    root = sysroot(target, sdk)
    library = sdk/'lib/x86_64-linux-gnu'
    directories = [library/'pkgconfig', sdk/'lib/pkgconfig', sdk/'share/pkgconfig']
    flags = [f'--sysroot={root}', '-isystem', str(root/'usr/include/x86_64-linux-gnu')]
    links = ['-C', f'link-arg=-Wl,-rpath,{codecs}/lib',
             '-C', 'link-arg=-Wl,--disable-new-dtags', '-C', f'link-arg=-Wl,--dynamic-linker={library}/ld-linux-x86-64.so.2',
             '-C', f'link-arg=-Wl,-rpath,{library}', '-C', f'link-arg=-Wl,-rpath-link,{library}',
             '-C', f'link-arg=-Wl,-rpath,{library}/pulseaudio',
             '-C', f'link-arg=-Wl,-rpath-link,{library}/pulseaudio']
    # Explicit ABI selection must not inherit a global loader override into host tools.
    if result.get('LD_LIBRARY_PATH') or result.get('LD_PRELOAD'):
        raise ValueError('clear global LD_LIBRARY_PATH/LD_PRELOAD before selecting the SDK ABI')
    existing = result.pop('CARGO_ENCODED_RUSTFLAGS', '')
    ordinary = shlex.split(result.pop('RUSTFLAGS', ''))
    target_flags = shlex.split(result.get('CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS', ''))
    result.update(CARGO_NET_OFFLINE='true', CARGO_TARGET_DIR=str(target),
                  PKG_CONFIG_PATH=os.pathsep.join(map(str, directories)),
                  PKG_CONFIG_LIBDIR=os.pathsep.join(map(str, directories)),
                  PKG_CONFIG_SYSROOT_DIR=str(root), LIBCLANG_PATH=str(libclang),
                  CFLAGS=shlex.join(flags), BINDGEN_EXTRA_CLANG_ARGS=shlex.join(flags),
                  CARGO_ENCODED_RUSTFLAGS='\x1f'.join([*filter(None, existing.split('\x1f')), *ordinary, *target_flags, *links]),
                  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS=shlex.join([*target_flags, *links]))
    result['SPA_PLUGIN_DIR'] = str(library/'spa-0.2')
    result['PIPEWIRE_MODULE_DIR'] = str(library/'pipewire-0.3')
    return result


def native_tools(target, search_path):
    directory = target/'host-tools'
    directory.mkdir(parents=True, exist_ok=True)
    for name in ('pipewire', 'pw-cat', 'pw-dump', 'pw-cli', 'pw-link'):
        native = shutil.which(name, path=search_path)
        if native is None or Path(native).parent == directory:
            raise ValueError(f'missing original host tool: {name}')
        native = str(Path(native).resolve())
        text = '#!/bin/sh\nunset SPA_PLUGIN_DIR PIPEWIRE_MODULE_DIR\nexec ' + shlex.quote(native) + ' "$@"\n'
        publish_tool(directory/name, text)
    return os.pathsep.join([str(directory), search_path])


def current_tool(path, text):
    if path.is_symlink():
        raise ValueError(f'foreign tool adapter retained: {path}')
    if not path.exists():
        return False
    if path.read_text() != text:
        raise ValueError(f'foreign tool adapter retained: {path}')
    return True


def publish_tool(path, text):
    if current_tool(path, text):
        return
    with tempfile.NamedTemporaryFile(mode='w', dir=path.parent) as stream:
        stream.write(text)
        stream.flush()
        os.fchmod(stream.fileno(), 0o755)
        try:
            os.link(stream.name, path)
        except FileExistsError:
            if not current_tool(path, text):
                raise ValueError(f'concurrent tool publication disappeared: {path}')


def command(extra):
    args = extra[1:] if extra[:1] == ['--'] else extra
    if not args:
        args = ['test', '-p', 'blent', '--all-features', '--no-fail-fast']
    if args[0] not in {'test', 'build', 'check', 'llvm-cov'}:
        raise ValueError('expected local cargo test/build/check/llvm-cov command')
    if args[0] == 'llvm-cov':
        if len(args) < 2 or args[1] not in {'test', 'run'}:
            raise ValueError('expected llvm-cov test/run workflow')
        return ['cargo', 'llvm-cov', args[1], '--offline', '--locked', *args[2:]]
    return ['cargo', args[0], '--offline', '--locked', *args[1:]]


def main():
    args = arguments()
    sdk, codecs, target, clang = (path.expanduser().resolve() for path in (args.sdk, args.codecs, args.target_dir, args.libclang))
    dependencies(sdk, codecs, clang)
    matching_codecs(sdk, codecs)
    env = environment(sdk, codecs, target, clang, os.environ)
    env['PATH'] = native_tools(target, env.get('PATH', os.defpath))
    invocation = command(args.cargo_args)
    print(f'SDK: {sdk}\nCodecs: {codecs}\nTarget/cache owner: {target}\nCommand: {shlex.join(invocation)}', flush=True)
    # Replace this process: terminal cancellation and original process-group ownership persist.
    os.execvpe(invocation[0], invocation, env)


if __name__ == '__main__':
    main()
