#!/usr/bin/env python3
"""T1670/T1590: finite synthetic camera decode and pipe-capacity observations.

Uses the current stock decoder/filter recipe. No tablet, camera, GPU or installed
configuration is opened. One decoder, one bounded consumer buffer and one private
encoded clip are owned at a time; all original children are waited on.
"""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import resource
import subprocess
import tempfile
import time

import ffmpeg_camera_latency


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ffmpeg', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--frames', type=int, choices=range(30, 301), default=120)
    parser.add_argument('--repeats', type=int, choices=range(1, 8), default=3)
    parser.add_argument('--sweep', choices=('pipes', 'filter-threads', 'latency'), default='pipes')
    return parser.parse_args()


def generate(ffmpeg, clip, width, height, frames):
    subprocess.run([str(ffmpeg), '-nostdin', '-hide_banner', '-loglevel', 'error',
                    '-f', 'lavfi', '-i', f'testsrc2=size={width}x{height}:rate=30',
                    '-frames:v', str(frames), '-c:v', 'libx264', '-preset', 'ultrafast',
                    '-tune', 'zerolatency', '-threads', '1', '-g', '30', '-bf', '0',
                    '-x264-params', 'aud=1',
                    '-color_primaries', 'bt709', '-color_trc', 'bt709',
                    '-colorspace', 'bt709', '-color_range', 'tv', '-f', 'h264', str(clip)],
                   stdin=subprocess.DEVNULL, check=True)


def recipe(ffmpeg, width, height, filter_threads):
    pixels = ((width + 63) // 64 * 64) * ((height + 63) // 64 * 64)
    filters = (f'scale={width}:{height}:force_original_aspect_ratio=decrease,'
               'colorspace=all=smpte170m:trc=bt709:range=tv:format=yuv420p,'
               f'pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,format=yuv420p')
    return [str(ffmpeg), '-nostdin', '-hide_banner', '-loglevel', 'error',
            '-max_alloc', '67108864', '-max_pixels', str(pixels), '-filter_threads', str(filter_threads),
            '-threads', '1', '-flags', 'low_delay', '-probesize', '32', '-analyzeduration', '0',
            '-f', 'h264', '-i', 'pipe:0', '-an', '-vf', filters, '-threads', '1',
            '-f', 'rawvideo', '-pix_fmt', 'yuv420p', 'pipe:1']


def capacity(pipe, requested):
    error = None
    try:
        fcntl.fcntl(pipe, fcntl.F_SETPIPE_SZ, requested)
    except OSError as cause:
        error = cause.errno
    return {'requested_bytes': requested,
            'effective_bytes': fcntl.fcntl(pipe, fcntl.F_GETPIPE_SZ), 'errno': error}


def consume(pipe, frame_bytes):
    buffer = bytearray(frame_bytes)
    view = memoryview(buffer)
    digest = hashlib.sha256()
    total = reads = 0
    while count := pipe.readinto(buffer):
        digest.update(view[:count])
        total += count
        reads += 1
    return {'bytes': total, 'reads': reads, 'sha256': digest.hexdigest()}


def child_metrics(before, after, elapsed):
    return {'wall_ms': elapsed * 1000,
            'child_user_ms': (after.ru_utime - before.ru_utime) * 1000,
            'child_system_ms': (after.ru_stime - before.ru_stime) * 1000,
            'child_voluntary_switches': after.ru_nvcsw - before.ru_nvcsw,
            'child_involuntary_switches': after.ru_nivcsw - before.ru_nivcsw}


def observe(command, clip, frame_bytes, pipe_bytes):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.perf_counter()
    with clip.open('rb') as source, tempfile.TemporaryFile() as errors:
        with subprocess.Popen(command, stdin=source, stdout=subprocess.PIPE,
                              stderr=errors, bufsize=0) as child:
            pipe = capacity(child.stdout, pipe_bytes)
            try:
                result = consume(child.stdout, frame_bytes)
            finally:
                child.stdout.close()
            code = child.wait()
        errors.seek(0)
        diagnostic = errors.read(65536).decode(errors='replace')
    if code:
        raise RuntimeError(f'FFmpeg exited {code}: {diagnostic}')
    result.update(child_metrics(before, resource.getrusage(resource.RUSAGE_CHILDREN),
                                time.perf_counter() - started))
    result.update(pipe)
    return result


def cases(args, root, width, height):
    clip = root / f'{width}x{height}.h264'
    generate(args.ffmpeg, clip, width, height, args.frames)
    frame_bytes = width * height * 3 // 2
    observations = []
    variants = [(65536, 1), (262144, 1), (1048576, 1)]
    if args.sweep != 'pipes':
        variants = [(65536, 1), (65536, 2), (65536, 4)]
    for repeat in range(args.repeats):
        # Rotate order to avoid assigning the first-run/cache effect to one size.
        for requested, threads in variants[repeat % 3:] + variants[:repeat % 3]:
            command = recipe(args.ffmpeg, width, height, threads)
            observer = ffmpeg_camera_latency.observe if args.sweep == 'latency' else observe
            row = observer(command, clip, frame_bytes, requested)
            row.update(filter_threads=threads, command=command)
            assert row['bytes'] == frame_bytes * args.frames
            observations.append(row)
            print(f'{width}x{height} pipe={row["effective_bytes"]} threads={threads}: '
                  f'{row["wall_ms"]:.1f} ms, {row["reads"]} reads', flush=True)
    assert len({row['sha256'] for row in observations}) == 1
    clip.unlink()
    return {'width': width, 'height': height, 'frames': args.frames,
            'observations': observations}


def main():
    args = arguments()
    report = {'version': subprocess.check_output([str(args.ffmpeg), '-version'], text=True).splitlines()[0],
              'scope': 'Finite file-fed synthetic H.264; one-thread decode, explicit filter-thread count; raw-output consumer hashes bytes. Throughput/child CPU evidence, not live camera latency.',
              'sweep': args.sweep, 'repeats': args.repeats}
    with tempfile.TemporaryDirectory(prefix='blent-ffmpeg-pipe-') as directory:
        report['cases'] = [cases(args, Path(directory), width, height)
                           for width, height in [(640, 480), (1280, 720), (1920, 1080)]]
    args.output.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
