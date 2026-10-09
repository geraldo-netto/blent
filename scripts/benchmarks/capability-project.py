#!/usr/bin/env python3
"""Build an isolated, shell-only metadata benchmark using cached dependencies."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def decoder_builder():
    spec = importlib.util.spec_from_file_location('decoder_project', ROOT / 'scripts/benchmarks/decoder-project.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def prepare(args):
    directory = decoder_builder().prepare(args)
    shutil.copy2(ROOT / 'android/local.properties', directory / 'local.properties')
    receiver = ROOT / 'scripts/benchmarks/android-capabilities/CapabilityReportReceiver.kt'
    shutil.copy2(receiver, directory / 'app/src/main/java/CapabilityReportReceiver.kt')
    manifest = directory / 'app/src/main/AndroidManifest.xml'
    declaration = ('<receiver android:name="com.blent.benchmark.CapabilityReportReceiver" '
                   'android:exported="true" android:permission="android.permission.DUMP" />')
    manifest.write_text(manifest.read_text().replace('</application>', declaration + '</application>'))
    return directory


def provenance(directory, args):
    apk = directory / 'app/build/outputs/apk/debug/app-debug.apk'
    originals = directory / 'originals'
    sources = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in originals.glob('*.kt')}
    receiver = directory / 'app/src/main/java/CapabilityReportReceiver.kt'
    result = dict(revision=args.revision or 'working-tree', package=args.package,
                  apk_sha256=hashlib.sha256(apk.read_bytes()).hexdigest(), sources=sources,
                  benchmark_sha256=hashlib.sha256(receiver.read_bytes()).hexdigest(), offline=True)
    (directory / 'capability-provenance.json').write_text(json.dumps(result, indent=2) + '\n')
    return apk


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--package', choices=['com.blent.capabilitybench.baseline', 'com.blent.capabilitybench.candidate'], required=True)
    parser.add_argument('--revision')
    args = parser.parse_args()
    if args.revision:
        args.revision = subprocess.check_output(['git', 'rev-parse', '--verify', args.revision + '^{commit}'], cwd=ROOT, text=True).strip()
    directory = prepare(args)
    subprocess.run([str(ROOT / 'android/gradlew'), '-p', str(directory), '--offline', '--no-daemon', ':app:assembleDebug'], check=True)
    print(provenance(directory, args))


if __name__ == '__main__':
    main()
