from pathlib import Path
import hashlib,json,sys
sys.path[:0]=['scripts/complexity']
from kotlin import ARTIFACTS,kotlin_scores
root=Path.cwd().resolve();cache=Path('/home/netto/.cache/blent-complexity')
assert all((cache/k.rsplit('/',1)[-1]).is_file() and hashlib.sha256((cache/k.rsplit('/',1)[-1]).read_bytes()).hexdigest()==v for k,v in ARTIFACTS.items()),'Required pinned offline Kotlin cache unavailable'
paths=[root/'android/app/src/test/java/com/blent/connection'/p for p in ['ConnectionSettingsContractTest.kt','AndroidConnectionPreferencesTest.kt']]
scores=kotlin_scores(paths)
Path('/tmp/blent-t898-evidence/kotlin-complexity.json').write_text(json.dumps(dict(tool='Kotlin PSI 1.9.20, repository Sonar-style source estimate; not native SonarQube',functions=scores),indent=2)+'\n')
assert max(row[3] for row in scores)<=9
print('Kotlin source audit',len(scores),'functions, maximum',max(row[3] for row in scores))
