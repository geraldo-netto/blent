#!/usr/bin/env python3
"""Build isolated audio CPU/ART probe from exact sources, using cached dependencies."""
import argparse
import importlib.util
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def direct_packets(directory):
    import re
    packets = (directory / 'originals/AudioPackets.kt').read_text()
    direct = packets.replace('class AudioPackets', 'class DirectAudioPackets')
    direct = re.sub(r'^internal data class (AudioBlock|PlaybackChunk).*\n', '', direct, flags=re.M)
    if 'val bytes = source.readByteArray(960L * direction)' in direct:
        direct = re.sub(r'            val bytes = source.readByteArray\(960L \* direction\).*?            previous =',
            '            val samples = ShortArray(480 * direction) { source.readShortLe() }\n            previous =', direct, flags=re.S)
    else:
        direct = direct.replace('val samples = readSamples(source)', 'val samples = ShortArray(480 * direction) { source.readShortLe() }')
    (directory / 'app/src/main/java/DirectAudioPackets.kt').write_text(direct)
    (directory / 'originals/DirectAudioPackets.kt').write_text(direct)


def pipeline_track(directory):
    typed = 'fun write(samples: ShortArray' in (directory / 'originals/AudioOutput.kt').read_text()
    parameter = 'samples: ShortArray' if typed else 'samples: ByteArray'
    value = 'samples[offset].toLong()' if typed else '((samples[offset].toInt() and 255) or ((samples[offset + 1].toInt() and 255) shl 8)).toShort().toLong()'
    track = """package com.blent.benchmark
import com.blent.*
internal class PipelineTrack : OutputTrack {
    var checksum = 0L
    override fun play() {}
    override fun flush() {}
    override fun route(): Int? = null
    override fun description() = "CPU fixture"
    override fun close() {}
    override fun write(PARAMETER, offset: Int, count: Int): OutputWrite {
        checksum += VALUE
        return OutputWrite(count)
    }
}
""".replace('PARAMETER', parameter).replace('VALUE', value)
    preferences = (directory / 'originals/AudioPreferences.kt').read_text()
    if 'speakerLowLatency' in preferences:
        track += '\ninternal fun pipelinePreferences(lowLatency: Boolean) = AudioPreferences(gain = 0, builtIn = false, speakerLowLatency = lowLatency)\n'
    else:
        track += '\ninternal fun pipelinePreferences(lowLatency: Boolean): AudioPreferences { require(!lowLatency); return AudioPreferences(gain = 0, builtIn = false) }\n'
    (directory / 'app/src/main/java/PipelineTrack.kt').write_text(track)
    (directory / 'originals/PipelineTrack.kt').write_text(track)


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
    for name in ['AudioPlaybackQueue.kt', 'AudioClock.kt', 'AudioPackets.kt', 'AudioOutput.kt', 'AudioRoute.kt', 'AudioSpeaker.kt', 'AudioPreferences.kt', 'AudioEndpoint.kt']:
        source = builder.source(name, args.revision)
        if source is None:
            raise SystemExit('Missing audio source: ' + name)
        (directory / 'originals' / name).write_text(source)
        (directory / 'app/src/main/java' / name).write_text(source)
    direct_packets(directory)
    pipeline_track(directory)
    for name in ['AudioPipelineProbe.kt', 'AudioModeProbe.kt']:
        text = (ROOT / 'scripts/benchmarks/android-audio' / name).read_text()
        if 'suspend fun awaitReady()' not in (directory / 'originals/AudioOutput.kt').read_text():
            text = text.replace('override suspend fun awaitReady() {}; ', '')
        (directory / 'app/src/main/java' / name).write_text(text)
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
        mode_sha256=hashlib.sha256((ROOT / "scripts/benchmarks/android-audio/AudioModeProbe.kt").read_bytes()).hexdigest(),
        pipeline_sha256=hashlib.sha256((ROOT / "scripts/benchmarks/android-audio/AudioPipelineProbe.kt").read_bytes()).hexdigest(),
        sources={p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (directory / 'originals').glob('*.kt')})
    (directory / 'audio-provenance.json').write_text(json.dumps(result, indent=2) + '\n')
    print(apk)


if __name__ == '__main__':
    main()
