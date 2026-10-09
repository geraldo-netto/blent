#!/usr/bin/env python3
"""Build isolated audio CPU/ART probe from exact sources, using cached dependencies."""
import argparse
import importlib.util
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--package', choices=['com.blent.audiobench.baseline', 'com.blent.audiobench.candidate'], required=True)
    parser.add_argument('--revision')
    args = parser.parse_args()
    if args.revision:
        args.revision = subprocess.check_output(['git', 'rev-parse', '--verify', args.revision + '^{commit}'], cwd=ROOT, text=True).strip()
    spec = importlib.util.spec_from_file_location('capability_project', ROOT / 'scripts/benchmarks/capability-project.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    builder = module.decoder_builder()
    directory = builder.prepare(args)
    shutil.copy2(ROOT / 'android/local.properties', directory / 'local.properties')
    for name in ['AudioPlaybackQueue.kt', 'AudioClock.kt', 'AudioPackets.kt']:
        source = builder.source(name, args.revision)
        if source is None:
            raise SystemExit('Missing audio source: ' + name)
        (directory / 'originals' / name).write_text(source)
        (directory / 'app/src/main/java' / name).write_text(source)
    receiver = ROOT / 'scripts/benchmarks/android-audio/AudioQueueReceiver.kt'
    shutil.copy2(receiver, directory / 'app/src/main/java/AudioQueueReceiver.kt')
    manifest = directory / 'app/src/main/AndroidManifest.xml'
    declaration = ('<receiver android:name="com.blent.benchmark.AudioQueueReceiver" '
                   'android:exported="true" android:permission="android.permission.DUMP" />')
    manifest.write_text(manifest.read_text().replace('</application>', declaration + '</application>'))
    build = directory / 'app/build.gradle.kts'
    build.write_text(build.read_text() + '\ndependencies { implementation("com.squareup.okhttp3:okhttp:4.12.0") }\n')
    subprocess.run([str(ROOT / 'android/gradlew'), '-p', str(directory), '--offline', '--no-daemon', ':app:assembleDebug'], check=True)
    # Reuse provenance hashing, naming the actual benchmark source explicitly.
    import hashlib, json
    apk = directory / 'app/build/outputs/apk/debug/app-debug.apk'
    result = dict(revision=args.revision or 'working-tree', package=args.package, offline=True,
        apk_sha256=hashlib.sha256(apk.read_bytes()).hexdigest(),
        benchmark_sha256=hashlib.sha256(receiver.read_bytes()).hexdigest(),
        sources={p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (directory / 'originals').glob('*.kt')})
    (directory / 'audio-provenance.json').write_text(json.dumps(result, indent=2) + '\n')
    print(apk)


if __name__ == '__main__':
    main()
