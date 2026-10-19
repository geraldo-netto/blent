from dataclasses import asdict
import hashlib, json, re, subprocess, sys
from pathlib import Path
sys.path[:0] = ['scripts/complexity', 'scripts/coverage']
from kotlin import ARTIFACTS, kotlin_scores
from kotlin_inventory import functions
from kotlin_report import read
from report import collect
base=Path('/tmp/blent-t1099-evidence')
root=Path.cwd().resolve()
cache=Path('/home/netto/.cache/blent-complexity')
assert all((cache/k.rsplit('/',1)[-1]).is_file() and hashlib.sha256((cache/k.rsplit('/',1)[-1]).read_bytes()).hexdigest()==v for k,v in ARTIFACTS.items())
production=['android/app/src/main/java/com/blent/'+p for p in ['MainActivity.kt',
    'connection/ConnectionControls.kt','connection/AndroidConnectionPreferences.kt','connection/ConfiguredConnection.kt',
    'connection/ConnectionPreferenceStore.kt','connection/ConnectionPreferenceForm.kt','connection/ConnectionPreferenceControls.kt']]
paths=production+['android/app/src/test/java/com/blent/connection/'+p for p in [
    'ConnectionPreferenceStoreTest.kt','ConnectionPreferenceLifetimeTest.kt','ConnectionPreferenceHistoryTest.kt',
    'ConfiguredConnectionTest.kt','ConnectionPreferenceFormTest.kt','AndroidConnectionPreferencesTest.kt',
    'ConnectionPreferenceControlsTest.kt','PreferenceStorageRig.kt','ConnectionControlsTest.kt',
    'ConnectionUiFixture.kt','ConnectionActivityTest.kt','ConnectionActivityUiTest.kt']]
new=set(production)-{'android/app/src/main/java/com/blent/MainActivity.kt','android/app/src/main/java/com/blent/connection/ConnectionControls.kt'}
if sys.argv[1]=='scope':
    changed={}
    for path in production:
        diff=subprocess.check_output(['git','diff','--unified=0','--',path],text=True)
        changed[path]=[(int(a),int(a)+max(int(b or '1'),1)-1) for a,b in re.findall(r'^@@ .*? \+(\d+)(?:,(\d+))? @@',diff,re.M)]
    inventory=[asdict(f) for f in functions(root,[root/p for p in production]) if f.file in new or any(a<=f.last and b>=f.first for a,b in changed[f.file])]
    (base/'inventory.json').write_text(json.dumps(inventory,indent=2)+'\n')
    scores=kotlin_scores([root/p for p in paths])
    (base/'complexity.json').write_text(json.dumps(dict(tool='Kotlin PSI 1.9.20, repository Sonar-style source estimate; not native SonarQube',functions=scores),indent=2)+'\n')
    assert max(row[3] for row in scores)<=9,scores
    snapshot={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(Path('android/app/src').rglob('*.kt'))}
    frozen=base/'sources.json'
    if frozen.exists():
        assert json.loads(frozen.read_text())==snapshot
    else:
        frozen.write_text(json.dumps(snapshot,indent=2)+'\n')
    print(len(inventory),'authored inventory entries;',len(scores),'source estimates; maximum',max(row[3] for row in scores))
else:
    frozen=json.loads((base/'sources.json').read_text())
    assert frozen=={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(Path('android/app/src').rglob('*.kt'))}
    report=root/'android/app/build/reports/jacoco/profileCoverage/profileCoverage.xml'
    assert report.stat().st_mtime>(base/'sources.json').stat().st_mtime
    rows=collect({'functions':json.loads((base/'inventory.json').read_text())},{},read(report,root/'android/app/src/main/java',root),[])
    (base/'per-function.json').write_text(json.dumps(dict(passes=all(r['passes'] for r in rows),functions=rows),indent=2)+'\n')
    for row in rows: print(row['name'],row['line'],row['percent'],row['evidence'])
    assert all(row['passes'] for row in rows),'Missing/below-80 native authored method counters'
