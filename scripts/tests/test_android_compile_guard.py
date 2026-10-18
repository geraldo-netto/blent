"""T1071: stale untouched callers cannot survive native test recompilation."""
from pathlib import Path
import os
import shutil
import tempfile
import unittest
import xml.etree.ElementTree as ET

import test_secure_connection_interop as interop

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path(__file__).with_name('fixtures') / 'kotlin-caller'


class AndroidCompileGuardTest(unittest.TestCase):
    def gradle(self, folder, task, phase):
        interop.SecureConnectionInteropTest().command([
            str(ROOT / 'android/gradlew'), '-p', str(folder), '--offline',
            '--no-daemon', '--console=plain', '-I', str(ROOT / 'scripts/coverage/android.init.gradle'),
            '-I', str(folder / 'seed-stale.init.gradle'), task,
        ], dict(os.environ), folder / (phase + '.log'))
        print('T1071 completed ' + phase, flush=True)

    def seed(self, folder):
        self.gradle(folder, ':app:compileDebugUnitTestKotlin', 'original-caller')
        original = folder / 'app/build/tmp/kotlin-classes/debugUnitTest/probe/UntouchedTest.class'
        shutil.copy2(original, folder / 'stale-caller.class')
        shutil.copy2(folder / 'current/Api.kt', folder / 'app/src/main/java/probe/Api.kt')
        flag = folder / 'inject-stale-caller'
        flag.touch()
        self.gradle(folder, ':app:compileDebugUnitTestKotlin', 'changed-api-stale-output')
        self.assertEqual((folder / 'stale-caller.class').read_bytes(), original.read_bytes())
        flag.unlink()

    def test_t1071_fresh_compilation_rebuilds_seeded_untouched_caller(self):
        with tempfile.TemporaryDirectory(prefix='blent-t1071-guard-') as temporary:
            folder = Path(temporary)
            shutil.copytree(FIXTURE, folder, dirs_exist_ok=True)
            local = ROOT / 'android/local.properties'
            if local.exists():
                shutil.copy2(local, folder / 'local.properties')
            self.seed(folder)
            # A prior compiler task recorded the stale class as its output.
            # Re-running the suite must freshly compile even unchanged sources.
            self.gradle(folder, ':app:testDebugUnitTest', 'unchanged-suite-cached-caller')
            suites = [ET.parse(path).getroot() for path in
                      (folder / 'app/build/test-results/testDebugUnitTest').glob('TEST-*.xml')]
            self.assertEqual(2, sum(int(suite.get('tests', 0)) for suite in suites))
            self.assertEqual(0, sum(int(suite.get('failures', 0)) for suite in suites))
            self.assertEqual(0, sum(int(suite.get('skipped', 0)) for suite in suites))


if __name__ == '__main__':
    unittest.main()
