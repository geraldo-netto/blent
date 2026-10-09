#!/usr/bin/env python3
"""Build shell-only synthetic Surface encoder probe offline; never opens a camera."""
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
    args.package = 'com.blent.camerabench'
    args.revision = None
    spec = importlib.util.spec_from_file_location('decoder', ROOT / 'scripts/benchmarks/decoder-project.py')
    builder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(builder)
    directory = builder.prepare(args)
    shutil.copy2(ROOT / 'android/local.properties', directory / 'local.properties')
    sources = {}
    for source in (ROOT / 'scripts/benchmarks/android-camera').glob('*.kt'):
        shutil.copy2(source, directory / 'app/src/main/java' / source.name)
        sources[source.name] = hashlib.sha256(source.read_bytes()).hexdigest()
    manifest = directory / 'app/src/main/AndroidManifest.xml'
    manifest.write_text(manifest.read_text().replace('</application>', '<receiver android:name="com.blent.benchmark.CameraProbeReceiver" android:exported="true" android:permission="android.permission.DUMP" /></application>'))
    subprocess.run([str(ROOT / 'android/gradlew'), '-p', str(directory), '--offline', '--no-daemon', ':app:assembleDebug'], check=True)
    apk = directory / 'app/build/outputs/apk/debug/app-debug.apk'
    result = dict(offline=True, sources=sources, apk_sha256=hashlib.sha256(apk.read_bytes()).hexdigest(),
                  camera_capture_sha256=hashlib.sha256((ROOT / 'android/app/src/main/java/com/blent/CameraCapture.kt').read_bytes()).hexdigest())
    (directory / 'camera-provenance.json').write_text(json.dumps(result, indent=2) + '\n')
    print(apk)


if __name__ == '__main__':
    main()
