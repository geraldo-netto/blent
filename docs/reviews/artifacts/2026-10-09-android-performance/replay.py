#!/usr/bin/env python3
"""Offline host-JVM review probe. Candidate code is generated only in a temp dir.

No Android/native throughput, power, or production-regression claim. Requires
the four already-cached Kotlin compiler jars; never downloads missing inputs.
"""
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
SOURCE = ROOT / 'android/app/src/main/java/com/blent'
CACHE = Path.home() / '.cache/blent-complexity'
JARS = ['kotlin-compiler-embeddable-1.9.20.jar', 'kotlin-stdlib-1.9.20.jar',
        'trove4j-1.0.20200330.jar', 'annotations-13.0.jar']


def replace_once(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)


def candidate(name, text):
    if name == 'AudioPlaybackQueue.kt':
        text = replace_once(text, 'PlaybackChunk(interpolate(step), at, discontinuity)',
                            'PlaybackChunk(if (ppm == 0 && phase == 0L && offset == 0) blocks.first.samples else interpolate(step), at, discontinuity)')
    if name == 'AudioClock.kt':
        text = replace_once(text, 'internal class AudioClockDrift {', '''internal class AudioClockDrift {
    private var sourceCache: Pair<Long, Long>? = null
    private var destinationCache: Pair<Long, Long>? = null
    private var resultCache = 0''')
        text = replace_once(text, 'fun clear() { source.clear(); destination.clear() }',
                            'fun clear() { source.clear(); destination.clear(); sourceCache = null; destinationCache = null }')
        text = replace_once(text, '        val numerator = BigInteger.valueOf(',
                            '''        if (sourceRate == sourceCache && destinationRate == destinationCache) return resultCache
        val numerator = BigInteger.valueOf(''')
        text = replace_once(text, '        return (difference / denominator).toInt()', '''        val result = (difference / denominator).toInt()
        sourceCache = sourceRate; destinationCache = destinationRate; resultCache = result
        return result''')
    return text


def compiler_classpath():
    for name in JARS:
        if not (CACHE / name).is_file():
            raise SystemExit(f'Missing cached compiler: {name}; no download attempted')
    return os.pathsep.join(str(CACHE / name) for name in JARS)


def metadata(sources):
    return {'scope': 'host JVM; synthetic steady audio; no native Android validation',
              'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
              'machine': platform.platform(),
              'java': subprocess.check_output(['java', '-version'], stderr=subprocess.STDOUT, text=True).strip(),
              'sources': {name: hashlib.sha256(text.encode()).hexdigest() for name, text in sources.items()},
              'compiler_jars': {name: hashlib.sha256((CACHE / name).read_bytes()).hexdigest() for name in JARS},
              'trials': []}


def build(folder, variant, sources, cp):
    path = Path(folder) / variant
    path.mkdir()
    for name, text in sources.items():
        (path / name).write_text(candidate(name, text) if variant == 'candidate' else text)
    (path / 'Probe.kt').write_bytes((HERE / 'Probe.kt').read_bytes())
    (path / 'MediaCodec.kt').write_text('package android.media\nclass MediaCodec { fun stop() {}; fun release() {} }\n')
    jar = path / 'probe.jar'
    subprocess.run(['java', '-cp', cp, 'org.jetbrains.kotlin.cli.jvm.K2JVMCompiler',
                    '-no-stdlib', '-no-reflect', '-cp', cp, '-d', str(jar),
                    *map(str, sorted(path.glob('*.kt')))], check=True, timeout=60)
    return jar


def trials(run, variants):
    rows = []
    for trial in range(5):
        for mode, count in [('queue', 50000), ('drift', 500000)]:
            signatures = []
            for variant in (variants if trial % 2 else list(reversed(variants))):
                fields = run(variant, mode, count)
                row = dict(zip(['count', 'wall_ns', 'cpu_ns', 'allocated_bytes', 'signature', 'checksum'], map(int, fields[1:])))
                row.update(variant=variant, mode=mode, trial=trial)
                rows.append(row)
                signatures.append(row['signature'])
            assert len(set(signatures)) == 1
    return rows


def summarize(rows):
    return {key + '_per_call': statistics.median(r[key] / r['count'] for r in rows)
            for key in ['wall_ns', 'cpu_ns', 'allocated_bytes']}


def medians(rows, variants):
    result = {}
    for mode in ['queue', 'drift']:
        result[mode] = {}
        for variant in variants:
            selected = [r for r in rows if r['mode'] == mode and r['variant'] == variant]
            result[mode][variant] = summarize(selected)
    return result


def main():
    cp = compiler_classpath()
    sources = {name: (SOURCE / name).read_text() for name in
               ['AudioPlaybackQueue.kt', 'AudioClock.kt', 'CodecLifetime.kt']}
    result = metadata(sources)
    with tempfile.TemporaryDirectory(prefix='blent-android-review-') as folder:
        jars = {variant: build(folder, variant, sources, cp) for variant in ['current', 'candidate']}

        def run(variant, mode, count=0):
            return subprocess.check_output(['java', '-Xms128m', '-Xmx128m', '-cp',
                                            str(jars[variant]) + os.pathsep + cp,
                                            'com.blent.ProbeKt', mode, str(count)], text=True, timeout=30).strip().split('\t')

        result['equivalence'] = {variant: run(variant, 'equivalence')[1] for variant in jars}
        assert len(set(result['equivalence'].values())) == 1
        result['retirement_wait_ns'] = int(run('current', 'retirement')[1])
        result['trials'] = trials(run, list(jars))
        result['medians'] = medians(result['trials'], jars)
    (HERE / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'medians': result['medians'], 'retirement_wait_ns': result['retirement_wait_ns'],
                      'equivalence': result['equivalence']}, indent=2))


if __name__ == '__main__':
    main()
