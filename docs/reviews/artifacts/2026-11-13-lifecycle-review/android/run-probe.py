"""Compile external review evidence against existing hosted classes; never touch tablet."""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[5]
ART = Path(__file__).resolve().parent
CACHE = Path.home() / '.gradle/caches/modules-2/files-2.1'
COMPILER_MODULES = [
    ('org.jetbrains.kotlin/kotlin-compiler-embeddable', '1.9.20'),
    ('org.jetbrains.kotlin/kotlin-stdlib', '1.9.20'),
    ('org.jetbrains.kotlin/kotlin-script-runtime', '1.9.20'),
    ('org.jetbrains.kotlin/kotlin-reflect', '1.6.10'),
    ('org.jetbrains.kotlin/kotlin-daemon-embeddable', '1.9.20'),
    ('org.jetbrains.intellij.deps/trove4j', '1.0.20200330'),
    ('org.jetbrains/annotations', '13.0'),
]
COMPILER = os.pathsep.join(str(next((CACHE / module / version).glob('*/*.jar'))) for module, version in COMPILER_MODULES)
JAVA = str(Path(os.environ['JAVA_HOME']) / 'bin/java')
WORK = Path(os.environ.get('BLENT_REVIEW_WORK', str(Path.home() / '.cache/blent-lifecycle-review-android')))
CLASSPATH = (WORK / 'classpath.txt').read_text()
COMPILE_CP = os.pathsep.join(value for value in CLASSPATH.split(os.pathsep) if 'test-parameter-injector' not in value and Path(value).exists())
OUTPUT = WORK / 'classes'
OUTPUT.mkdir(exist_ok=True)
COMPILE = [JAVA, '-cp', COMPILER, 'org.jetbrains.kotlin.cli.jvm.K2JVMCompiler', '-no-stdlib', '-no-reflect', '-jvm-target', '17', '-classpath', COMPILE_CP, '-Xfriend-paths=' + COMPILE_CP.replace(os.pathsep, ','), '-d', str(OUTPUT), str(ART / 'ReviewAdbCutoffProbe.kt'), str(ART / 'ReviewDisplayReplacementProbe.kt')]
with (ART / 'compile.log').open('w') as log:
    subprocess.run(COMPILE, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
with (ART / 'probe.log').open('w') as log:
    result = subprocess.run([JAVA, '-Xmx2048m', '--add-opens=java.base/java.io=ALL-UNNAMED', '-cp', str(OUTPUT) + os.pathsep + CLASSPATH, 'org.junit.runner.JUnitCore', 'com.blent.ReviewAdbCutoffProbe', 'com.blent.ReviewDisplayReplacementProbe'], cwd=ROOT / 'android/app', stdout=log, stderr=subprocess.STDOUT)
print('probe exit:', result.returncode)
