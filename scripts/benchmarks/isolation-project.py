#!/usr/bin/env python3
"""Build disposable native process-containment experiment offline."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    args = parser.parse_args()
    args.package = 'com.blent.isolationprobe'
    args.revision = None
    spec = importlib.util.spec_from_file_location('decoder', ROOT / 'scripts/benchmarks/decoder-project.py')
    builder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(builder)
    directory = builder.prepare(args)
    shutil.copy2(ROOT / 'android/local.properties', directory / 'local.properties')
    sources = {}
    for source in (ROOT / 'scripts/benchmarks/android-isolation').glob('*.kt'):
        shutil.copy2(source, directory / 'app/src/main/java' / source.name)
        sources[source.name] = hashlib.sha256(source.read_bytes()).hexdigest()
    declarations = ['<receiver android:name="com.blent.benchmark.IsolationProbe" android:exported="true" android:permission="android.permission.DUMP" />']
    for name, process in [('Dedicated0', 'r0'), ('Dedicated1', 'r1'), ('Dedicated2', 'r2'), ('Dedicated3', 'r3'), ('Grouped0', 'group'), ('Grouped1', 'group')]:
        declarations.append(f'<service android:name="com.blent.benchmark.{name}" android:process=":{process}" android:exported="false" />')
    manifest = directory / 'app/src/main/AndroidManifest.xml'
    manifest.write_text(manifest.read_text().replace('</application>', ''.join(declarations) + '</application>'))
    subprocess.run([str(ROOT / 'android/gradlew'), '-p', str(directory), '--offline', '--no-daemon', ':app:assembleDebug'], check=True)
    apk = directory / 'app/build/outputs/apk/debug/app-debug.apk'
    (directory / 'isolation-provenance.json').write_text(json.dumps(dict(offline=True, sources=sources, apk_sha256=hashlib.sha256(apk.read_bytes()).hexdigest()), indent=2) + '\n')
    print(apk)


if __name__ == '__main__':
    main()
