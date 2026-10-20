"""T835: explicit SDK ABI, retained cache ownership, and offline commands."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('host_offline', ROOT/'scripts/ci/test-host-offline.py')
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class HostOfflineTest(unittest.TestCase):
    def test_t835_default_and_explicit_commands_are_offline_and_locked(self):
        self.assertEqual(runner.command([]), ['cargo','test','--offline','--locked','-p','blent','--all-features','--no-fail-fast'])
        self.assertEqual(runner.command(['--','build','-j','2']), ['cargo','build','--offline','--locked','-j','2'])
        self.assertEqual(runner.command(['llvm-cov','test','-p','blent']), ['cargo','llvm-cov','test','--offline','--locked','-p','blent'])
        for args in [ ['fetch'], ['install'], ['llvm-cov'], ['llvm-cov','report'] ]:
            with self.subTest(args=args), self.assertRaises(ValueError): runner.command(args)

    def test_t835_reuses_only_the_exact_sdk_prefix(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); sdk=root/'sdk space'; sdk.mkdir(); target=root/'target'
            self.assertEqual(runner.sysroot(target,sdk), runner.sysroot(target,sdk))
            prefix=target/'sdk-sysroot/usr'; self.assertEqual(prefix.resolve(),sdk)
            prefix.unlink(); prefix.symlink_to(root/'foreign')
            with self.assertRaises(ValueError): runner.sysroot(target,sdk)
            self.assertEqual(os.readlink(prefix),str(root/'foreign'))

    def test_t835_sdk_flags_preserve_argument_boundaries_and_host_tools(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); sdk=root/'sdk space'; sdk.mkdir()
            before={'PATH':'native tools','RUSTFLAGS':'-C debuginfo=1','CARGO_ENCODED_RUSTFLAGS':'-C\x1fopt-level=1'}
            env=runner.environment(sdk,root/'codecs',root/'build space',root/'clang',before)
            self.assertEqual(before['RUSTFLAGS'],'-C debuginfo=1')
            self.assertEqual(env['PATH'],'native tools')
            self.assertEqual(env['CARGO_NET_OFFLINE'],'true')
            self.assertNotIn('LD_LIBRARY_PATH',env)
            self.assertIn(f'link-arg=-Wl,-rpath,{root}/codecs/lib',env['CARGO_ENCODED_RUSTFLAGS'].split('\x1f'))
            self.assertNotIn('RUSTFLAGS',env)
            self.assertIn('link-arg=-Wl,--disable-new-dtags',env['CARGO_ENCODED_RUSTFLAGS'].split('\x1f'))
            self.assertIn(f'link-arg=-Wl,-rpath,{sdk}/lib/x86_64-linux-gnu',env['CARGO_ENCODED_RUSTFLAGS'].split('\x1f'))
            self.assertIn(f'link-arg=-Wl,-rpath,{sdk}/lib/x86_64-linux-gnu/pulseaudio',env['CARGO_ENCODED_RUSTFLAGS'].split('\x1f'))
            self.assertIn('CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS',env)
            self.assertIn('--dynamic-linker=',env['CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS'])
            self.assertEqual(env['BINDGEN_EXTRA_CLANG_ARGS'],env['CFLAGS'])

    def test_t835_global_loader_overrides_are_rejected(self):
        for name in ['LD_LIBRARY_PATH','LD_PRELOAD']:
            with self.subTest(name=name),tempfile.TemporaryDirectory() as tmp:
                root=Path(tmp)
                with self.assertRaises(ValueError): runner.environment(root,root/'codecs',root/'build',root,{name:'foreign'})

    def test_t835_tool_adapter_executes_original_with_its_own_plugin_environment(self):
        import subprocess
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); native=root/'original'; native.mkdir()
            for name in ['pipewire','pw-cat','pw-dump','pw-cli','pw-link']:
                tool=native/name
                tool.write_text('#!/bin/sh\nprintf "%s\\n" "${SPA_PLUGIN_DIR-unset}" "${PIPEWIRE_MODULE_DIR-unset}" "$@"\n')
                tool.chmod(0o755)
            path=runner.native_tools(root/'cache',str(native))
            env=dict(os.environ,PATH=path,SPA_PLUGIN_DIR='foreign SDK',PIPEWIRE_MODULE_DIR='foreign SDK')
            result=subprocess.run([str(root/'cache/host-tools/pipewire'),'literal $value'],env=env,capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual(result.stdout.splitlines(),['unset','unset','literal $value'])
            runner.native_tools(root/'cache',str(native))
            adapter=root/'cache/host-tools/pipewire'; adapter.unlink(); adapter.symlink_to(native/'pipewire')
            with self.assertRaises(ValueError): runner.native_tools(root/'cache',str(native))

    def test_t835_missing_original_tool_never_creates_a_replacement(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            with self.assertRaises(ValueError): runner.native_tools(root/'cache',str(root/'absent'))
            self.assertEqual(list((root/'cache/host-tools').iterdir()),[])

    def test_t835_tool_publication_reuses_original_and_settles_failed_staging(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); path=root/'tool'
            runner.publish_tool(path,'original')
            original=path.stat().st_ino
            with patch.object(runner.os,'link',side_effect=OSError('denied')):
                runner.publish_tool(path,'original')
                with self.assertRaises(OSError): runner.publish_tool(root/'other','new')
            self.assertEqual(path.stat().st_ino,original)
            self.assertEqual(path.read_text(),'original')
            self.assertEqual(sorted(p.name for p in root.iterdir()),['tool'])
            with self.assertRaises(ValueError): runner.publish_tool(path,'foreign replacement')

    def test_t835_codecs_require_matching_cached_sdk_versions(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); sdk=root/'sdk'; codecs=root/'codecs'
            library=sdk/'lib/x86_64-linux-gnu'; library.mkdir(parents=True)
            (codecs/'lib').mkdir(parents=True)
            for name in ['avcodec','avutil','avdevice','avformat','avfilter','swresample','swscale']:
                real=f'lib{name}.so.61.19.101'
                (library/real).write_text('SDK fixture')
                (library/f'lib{name}.so').symlink_to(real)
                (codecs/'lib'/real).write_text('matching codec fixture')
            runner.matching_codecs(sdk,codecs)
            (codecs/'lib/libavcodec.so.61.19.101').unlink()
            (codecs/'lib/libavcodec.so.61.19.100').write_text('wrong version')
            with self.assertRaises(ValueError): runner.matching_codecs(sdk,codecs)

    def test_t835_concurrent_tool_winner_is_validated_without_replacement(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); path=root/'tool'
            def winner(_source, destination):
                Path(destination).write_text('original')
                raise FileExistsError()
            with patch.object(runner.os,'link',side_effect=winner):
                runner.publish_tool(path,'original')
            self.assertEqual(path.read_text(),'original')
            self.assertEqual(len(list(root.iterdir())),1)
            path.unlink()
            with patch.object(runner.os,'link',side_effect=winner):
                with self.assertRaises(ValueError): runner.publish_tool(path,'foreign')
            self.assertEqual(path.read_text(),'original')
            self.assertEqual(len(list(root.iterdir())),1)
