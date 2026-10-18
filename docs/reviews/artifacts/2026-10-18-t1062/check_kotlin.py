"""T1062: unchanged repository authored-method inventory and native JaCoCo mapping."""
from dataclasses import asdict
import hashlib
import json
from pathlib import Path
import sys
sys.path.insert(0, 'scripts/coverage')
sys.path.insert(0, 'scripts/complexity')
from kotlin import ARTIFACTS
from kotlin_inventory import functions
from kotlin_report import assign_methods, function_key, function_result, read

root = Path.cwd().resolve()
base = Path('/tmp/blent-t1062-evidence')
cache = Path('/home/netto/.cache/blent-complexity')
assert all((cache/key.rsplit('/',1)[-1]).is_file() and hashlib.sha256((cache/key.rsplit('/',1)[-1]).read_bytes()).hexdigest()==digest for key,digest in ARTIFACTS.items())
expected = json.loads((base/'sources.json').read_text())
current = {str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(Path('android/app/src').rglob('*.kt'))}
current['testdata/display-activity-v1.json'] = hashlib.sha256(Path('testdata/display-activity-v1.json').read_bytes()).hexdigest()
assert current == expected, 'Kotlin sources changed during native collection'
paths = [root/path for path in json.loads((base/'kotlin-scope.json').read_text())]

inventory = functions(root, paths)
report = root/'android/app/build/reports/jacoco/profileCoverage/profileCoverage.xml'
assert report.stat().st_mtime >= (base/'sources.json').stat().st_mtime, 'native report is stale'
methods, lines = read(report, root/'android/app/src/main/java', root)
assigned = assign_methods(inventory, methods)
rows = []
for function in inventory:
    native = assigned.get(function_key(function), [])
    if function.name == '<lambda>' and not native and function.file in lines:
        continue  # Existing supported inline-body attribution; owner keeps counters.
    rows.append(function_result(function, native, lines))
output = dict(passes=bool(rows) and all(row['passes'] for row in rows), functions=rows)
(base/'inventory.json').write_text(json.dumps([asdict(f) for f in inventory],indent=2)+'\n')
(base/'per-function.json').write_text(json.dumps(output,indent=2)+'\n')
for row in rows:
    print(row['name'], row['line'], row['percent'], row['evidence'])
assert output['passes'], 'authored method below 80% or missing native counters'
