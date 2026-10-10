#!/usr/bin/env python3
"""T817: local stock FFmpeg camera decode/readback replay; no camera devices.
Usage: camera-decode.py OUTPUT [--frames 60] [--pairs 3]
Synthetic sources and evidence stay under OUTPUT. No downloads or installation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import statistics
import subprocess
import threading
import time


def source(root, width, height, frames):
    path = root / f'{width}x{height}.h264'
    subprocess.run(['ffmpeg', '-nostdin', '-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i', f'testsrc2=size={width}x{height}:rate=30', '-frames:v', str(frames), '-c:v', 'libx264', '-preset', 'ultrafast', '-tune', 'zerolatency', '-threads', '1', '-x264-params', 'aud=1:keyint=30', '-f', 'h264', str(path)], check=True)
    raw = path.read_bytes()
    packets = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-show_packets', '-show_entries', 'packet=pos,size', '-of', 'json', str(path)]))['packets']
    return [raw[int(p['pos']):int(p['pos']) + int(p['size'])] for p in packets]


def command(width, height, mode, rotation):
    prefix = ['ffmpeg', '-nostdin', '-hide_banner', '-loglevel', 'error', '-max_alloc', '67108864', '-max_pixels', str(((width+63)//64*64)*((height+63)//64*64)), '-filter_threads', '1', '-threads', '1', '-flags', 'low_delay', '-probesize', '32', '-analyzeduration', '0']
    if mode != 'software':
        prefix += ['-hwaccel', 'vaapi', '-hwaccel_device', mode.split(':')[0], '-hwaccel_output_format', 'vaapi']
    filters = ('transpose=clock,' if rotation else '') + f'scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,format=yuv420p'
    if mode.endswith(':scale'):
        assert not rotation
        filters = f'scale_vaapi=w={width}:h={height}:format=nv12,hwdownload,format=nv12,format=yuv420p'
    elif mode != 'software':
        filters = 'hwdownload,format=nv12,' + filters
    return prefix + ['-f', 'h264', '-i', 'pipe:0', '-an', '-vf', filters, '-threads', '1', '-f', 'rawvideo', '-pix_fmt', 'yuv420p', 'pipe:1']


def read_frame(stream, size):
    data = bytearray()
    while len(data) < size:
        part = stream.read(size-len(data))
        if not part:
            return bytes(data)
        data.extend(part)
    return bytes(data)


def replay(root, packets, width, height, mode, rotation, serial):
    cmd = command(width, height, mode, rotation)
    errors = root / f'run-{serial}.stderr'
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    sent, received, rss = [], [], []
    digest = hashlib.sha256()
    started = time.monotonic_ns()
    with errors.open('wb') as log:
        process = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log)
        def feed():
            try:
                for index, packet in enumerate(packets):
                    due = started + index * 1_000_000_000 // 30
                    time.sleep(max(0, (due-time.monotonic_ns()) / 1e9))
                    sent.append(time.monotonic_ns())
                    process.stdin.write(packet)
                    process.stdin.flush()
            except BrokenPipeError:
                pass
            finally:
                process.stdin.close()
        writer = threading.Thread(target=feed)
        writer.start()
        timer = threading.Timer(len(packets)/30+10, process.kill)
        timer.start()
        try:
            while True:
                data = read_frame(process.stdout, width*height*3//2)
                if not data:
                    break
                assert len(data) == width*height*3//2
                received.append(time.monotonic_ns())
                digest.update(data)
                try:
                    status = Path(f'/proc/{process.pid}/status').read_text()
                    rss.append(int(status.split('VmRSS:')[1].split()[0]))
                except (FileNotFoundError, IndexError):
                    pass
            writer.join(timeout=2)
            status = process.wait(timeout=2)
        finally:
            timer.cancel()
            if process.poll() is None:
                process.kill()
                process.wait()
            process.stdout.close()
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    ages = sorted((at-sent[i])/1e6 for i, at in enumerate(received))
    return dict(command=cmd, mode=mode, rotation=rotation, width=width, height=height,
                returncode=status, frames=len(received), input_frames=len(packets),
                sha256=digest.hexdigest(), startup_ms=(received[0]-started)/1e6 if received else None,
                age_p50_ms=statistics.median(ages) if ages else None,
                age_p95_ms=ages[min(len(ages)-1,len(ages)*95//100)] if ages else None,
                cpu_ms=((after.ru_utime+after.ru_stime)-(before.ru_utime+before.ru_stime))*1000,
                rss_peak_kib=max(rss,default=0), stderr=errors.read_text())


def modes_for(rotation):
    modes = ['software', '/dev/dri/renderD128', '/dev/dri/renderD129']
    return modes if rotation else modes + ['/dev/dri/renderD128:scale', '/dev/dri/renderD129:scale']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--frames', type=int, default=60)
    parser.add_argument('--pairs', type=int, default=3)
    args = parser.parse_args()
    if not 4 <= args.frames <= 300 or not 1 <= args.pairs <= 9:
        parser.error('frames 4..300; pairs 1..9')
    args.output.mkdir(parents=True, exist_ok=False)
    rows = []
    for width,height in [(1280,720),(1920,1080)]:
        packets = source(args.output,width,height,args.frames)
        for rotation in [False,True]:
            modes = modes_for(rotation)
            for pair in range(args.pairs):
                for mode in modes if pair%2==0 else list(reversed(modes)):
                    row=replay(args.output,packets,width,height,mode,rotation,len(rows))
                    row['pair']=pair
                    rows.append(row)
                    (args.output/'results.json').write_text(json.dumps(rows,indent=2)+'\n')
                    print(json.dumps({k:v for k,v in row.items() if k not in ['command','stderr']}),flush=True)
    (args.output/'environment.txt').write_text(subprocess.check_output(['ffmpeg','-version']).decode())


if __name__ == '__main__':
    main()
