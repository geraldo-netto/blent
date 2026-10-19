from pathlib import Path
import os,sys
sys.path.insert(0,'scripts/tests')
from test_secure_connection_interop import SecureConnectionInteropTest
path=Path('android/app/src/main/java/com/blent/connection/ConnectionPreferences.kt')
original=path.read_text()
fault=original.replace("text.length in 1..7 && text.all", "text.all").replace('        require(fields.size == ConnectionPreferenceField.entries.size) { "Incomplete USB settings" }\n','')
assert fault!=original
try:
    path.write_text(fault)
    try:
        SecureConnectionInteropTest().command([
            str(Path('android/gradlew').resolve()),'-p',str(Path('android').resolve()),'--offline','--no-daemon','--max-workers=2',
            '-I',str(Path('scripts/coverage/android.init.gradle').resolve()),':app:testDebugUnitTest',
            '--tests','com.blent.connection.ConnectionPreferencesTest.t1098InvalidNumericTextAndBoundedOutOfRangeValuesRejectAtomically',
            '--tests','com.blent.connection.ConnectionPreferencesTest.t1098UnknownMissingAndRetainedMutableInputCannotAlterSnapshot',
        ],dict(os.environ),Path('/tmp/blent-t1098-evidence/fault-red.log'))
    except AssertionError:
        log=Path('/tmp/blent-t1098-evidence/fault-red.log').read_text()
        assert '2 tests completed, 2 failed' in log,log[-3000:]
    else:
        raise AssertionError('Faulty controls unexpectedly passed')
finally:
    path.write_text(original)
print('Both faulty controls rejected; exact production source restored with fresh timestamp')
