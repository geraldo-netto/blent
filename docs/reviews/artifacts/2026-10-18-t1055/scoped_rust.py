from pathlib import Path
s=Path('/tmp/blent-t1016-evidence/scoped_rust.py').read_text();a=s.index('FILES =');b=s.index('\ndef snapshot',a)
s=s[:a]+'''FILES = ['common/src/display_activity.rs','common/src/display_activity/projection.rs']
TESTS = ['common/src/display_activity/tests.rs','common/src/display_activity/projection/tests.rs']
SELECTED = {}
BASE = Path('/tmp/blent-t1055-evidence')
''' +s[b:]
a=s.index('def runner(');b=s.index('output = BASE / sys.argv[1]',a);s=s[:a]+s[b:]
s=s.replace("Selection(packages=('blent',), binaries=True, features=('accessory-transport',), filter='capture::'), runner=runner, snapshotter=snapshot)", "Selection(packages=('blent-config',), library=True), snapshotter=snapshot)")
s = s.replace("hashes = {str(path): fingerprint(root / path) for path in paths}", "hashes = {str(path): fingerprint(root / path) for path in paths}\n    hashes['testdata/display-activity-v1.json'] = fingerprint(root / 'testdata/display-activity-v1.json')")
exec(compile(s, __file__, 'exec'))
