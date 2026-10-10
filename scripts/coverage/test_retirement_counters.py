"""T841: live retirement owners must export strict, unmodified LLVM counters."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import readers

ROOT = Path(__file__).resolve().parents[2]


def run(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=60, **kwargs)


class RetirementCountersTest(unittest.TestCase):
    def test_t841_idle_observer_exports_valid_native_counters(self):
        # Compile the actual observer, not a copied algorithm. Only its tracing
        # log macro is stubbed so this native regression needs no Cargo download.
        sysroot = Path(run('rustc', '--print', 'sysroot').stdout.strip())
        host = next(line.split(': ', 1)[1] for line in run('rustc', '-vV').stdout.splitlines()
                    if line.startswith('host: '))
        suffix = '.exe' if os.name == 'nt' else ''
        llvm = sysroot / 'lib/rustlib' / host / 'bin'
        with tempfile.TemporaryDirectory(prefix='blent-t841-') as directory:
            work = Path(directory)
            source = work / 'probe.rs'
            source.write_text('extern crate self as tracing;\n'
                              '#[macro_export] macro_rules! error { ($($t:tt)*) => {} }\n'
                              f'#[path = {json.dumps(str(ROOT / "common/src/retirement.rs"))}] mod retirement;\n'
                              'fn main() {\n'
                              '    let pool = retirement::Pool::new(1).unwrap();\n'
                              '    assert_eq!(pool.capacity(), 1);\n'
                              '    std::thread::sleep(std::time::Duration::from_millis(100));\n'
                              '    std::mem::forget(pool);\n'
                              '}\n')
            binary = work / ('probe' + suffix)
            run('rustc', '--edition=2021', '-Awarnings', '-Cinstrument-coverage',
                str(source), '-o', str(binary))
            raw, profile, lcov = work / 'raw.profraw', work / 'data.profdata', work / 'report.lcov'
            run(str(binary), env={**os.environ, 'LLVM_PROFILE_FILE': str(raw)})
            run(str(llvm / ('llvm-profdata' + suffix)), 'merge', '-sparse', str(raw), '-o', str(profile))
            export = run(str(llvm / ('llvm-cov' + suffix)), 'export', '-format=lcov',
                         '-instr-profile=' + str(profile), str(binary))
            lcov.write_text(export.stdout)
            if evidence := os.environ.get('BLENT_T841_LCOV'):
                Path(evidence).write_text(export.stdout)
            counters = {}
            readers.lcov(lcov, ROOT, counters)
            self.assertIn('common/src/retirement.rs', counters)
            reap_entry = next(number for number, line in enumerate(
                (ROOT / 'common/src/retirement.rs').read_text().splitlines(), 1)
                if line.startswith('fn reap('))
            # A starved observer must fail the test, not produce a false pass.
            self.assertGreater(counters['common/src/retirement.rs'][reap_entry], 0)
