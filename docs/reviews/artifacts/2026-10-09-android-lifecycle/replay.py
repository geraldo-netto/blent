#!/usr/bin/env python3
"""Offline fault-injection review; actual owners, fake platform/capture adapters."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
CACHE = Path.home() / '.cache/blent-complexity'
SOURCES = ['AudioBinding.kt', 'CameraBinding.kt', 'CameraResources.kt']
STUBS = {
    'Context.kt': 'package android.content\nopen class Context { fun checkSelfPermission(value: String) = 0 }',
    'Manifest.kt': 'package android\nobject Manifest { object permission { const val RECORD_AUDIO = "mic"; const val CAMERA = "camera" } }',
    'Permissions.kt': 'package android.content.pm\nobject PackageManager { const val PERMISSION_GRANTED = 0 }',
    'Handler.kt': 'package android.os\nclass Looper { companion object { fun getMainLooper() = Looper() } }; class Handler(looper: Looper)',
    'Dispatcher.kt': 'package kotlinx.coroutines.android\nfun android.os.Handler.asCoroutineDispatcher(): kotlinx.coroutines.CoroutineDispatcher = kotlinx.coroutines.Dispatchers.Unconfined',
    'Compose.kt': '''package androidx.compose.runtime
import kotlin.reflect.KProperty
class State<T>(var value: T)
fun <T> mutableStateOf(value: T) = State(value)
operator fun <T> State<T>.getValue(owner: Any?, key: KProperty<*>) = value
operator fun <T> State<T>.setValue(owner: Any?, key: KProperty<*>, next: T) { value = next }
''',
}


def classpath():
    files = [CACHE / name for name in ['kotlin-compiler-embeddable-1.9.20.jar',
             'kotlin-stdlib-1.9.20.jar', 'trove4j-1.0.20200330.jar', 'annotations-13.0.jar']]
    coroutine_root = Path.home() / '.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlinx/kotlinx-coroutines-core-jvm/1.7.3'
    coroutines = sorted(coroutine_root.glob('*/*.jar'))
    if not coroutines or not all(path.is_file() for path in files):
        raise SystemExit('Missing local compiler/coroutines cache; no download attempted')
    return files + [coroutines[0]]


def prepare(path):
    hashes = {}
    for name in SOURCES:
        content = (ROOT / 'android/app/src/main/java/com/blent' / name).read_bytes()
        hashes[name] = hashlib.sha256(content).hexdigest()
        (path / name).write_bytes(content)
    for name in ['Probe.kt', 'Stubs.kt']:
        (path / name).write_bytes((HERE / name).read_bytes())
    for name, content in STUBS.items():
        (path / name).write_text(content)
    return hashes


def main():
    files = classpath()
    cp = os.pathsep.join(map(str, files))
    with tempfile.TemporaryDirectory(prefix='blent-lifecycle-review-') as folder:
        path = Path(folder)
        hashes = prepare(path)
        jar = path / 'probe.jar'
        subprocess.run(['java', '-cp', cp, 'org.jetbrains.kotlin.cli.jvm.K2JVMCompiler',
                        '-nowarn', '-no-stdlib', '-no-reflect', '-cp', cp, '-d', str(jar),
                        *map(str, sorted(path.glob('*.kt')))], check=True, timeout=60)
        output = subprocess.check_output(['java', '-cp', str(jar) + os.pathsep + cp,
                                          'com.blent.ProbeKt'], text=True, timeout=40)
    result = dict(scope='Host JVM; production owners with fake capture/platform adapters',
                  head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                  sources=hashes, jars={p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
                  java=subprocess.check_output(['java', '-version'], stderr=subprocess.STDOUT, text=True).strip(),
                  output=output.splitlines())
    (HERE / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    print(output, end='')


if __name__ == '__main__':
    main()
