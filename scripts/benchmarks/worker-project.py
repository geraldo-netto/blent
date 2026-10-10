#!/usr/bin/env python3
"""Offline disposable native acceptance app using production worker IPC/ownership."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    args = parser.parse_args()
    directory = args.directory.resolve()
    directory.mkdir()
    android = directory / 'android'
    (android / 'app/src/main').mkdir(parents=True)
    for name in ['build.gradle.kts', 'settings.gradle.kts', 'gradle.properties', 'local.properties']:
        shutil.copy2(ROOT / 'android' / name, android / name)
    shutil.copytree(ROOT / 'android/gradle', android / 'gradle')
    shutil.copytree(ROOT / 'android/app/src/main', android / 'app/src/main', dirs_exist_ok=True)
    for name in ['build.gradle.kts', 'proguard-rules.pro']:
        shutil.copy2(ROOT / 'android/app' / name, android / 'app' / name)
    build = android / 'app/build.gradle.kts'
    build.write_text(build.read_text().replace('applicationId = "io.github.geraldo_netto.blent"', 'applicationId = "com.blent.workertest"'))
    sources = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT / 'android/app/src/main').rglob('*') if p.is_file()}
    native = android / 'app/src/main/java/com/blent/MediaWorkerService.kt'
    original = 'LocalMediaRuns.create(this, task, surface, report)'
    assert native.read_text().count(original) == 1
    native.write_text(native.read_text().replace(original, 'WorkerAcceptanceFactory.create(this, task, surface, report)'))
    for path in (ROOT / 'scripts/benchmarks/android-worker-tests').glob('*.kt'):
        shutil.copy2(path, android / 'app/src/main/java/com/blent' / path.name)
        sources[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest = android / 'app/src/main/AndroidManifest.xml'
    manifest.write_text(manifest.read_text().replace('</application>', '<receiver android:name="com.blent.WorkerAcceptanceReceiver" android:exported="true" android:permission="android.permission.DUMP" /></application>'))
    subprocess.run([str(ROOT / 'android/gradlew'), '-p', str(android), '--offline', '--no-daemon', ':app:assembleDebug'], check=True)
    apk = android / 'app/build/outputs/apk/debug/app-debug.apk'
    result = dict(package="com.blent.workertest", offline=True, sources=sources, apk_sha256=hashlib.sha256(apk.read_bytes()).hexdigest(),
                  factory_substitution='MediaWorkerService.createRun only: production IPC/supervision unchanged; fake native holds/death for a test credential, stock native metadata otherwise')
    (directory / 'worker-provenance.json').write_text(json.dumps(result, indent=2) + '\n')
    print(apk)


if __name__ == '__main__':
    main()
