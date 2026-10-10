#!/usr/bin/env python3
"""T817: compare preview progress with a deliberately unread output pipe.
A fused FFmpeg split shares synchronous output backpressure. The existing decoded
latest-value fanout can retain one bounded sink frame while preview keeps moving.
No real V4L2 node is opened. This is a pipe-stall replay, not V4L2 acceptance.
"""
import importlib.util
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time

spec = importlib.util.spec_from_file_location('camera_decode', Path(__file__).with_name('camera-decode.py'))
decode = importlib.util.module_from_spec(spec)
spec.loader.exec_module(decode)


def trial(path, fused):
    read_fd, write_fd = os.pipe()
    base = decode.command(160, 120, 'software', False)
    base[base.index('pipe:0')] = str(path)
    if fused:
        at = base.index('-vf')
        filters = base[at + 1]
        base = base[:at] + ['-filter_complex_threads', '1', '-filter_complex', '[0:v]'+filters+',split=2[p][s]', '-map', '[p]', '-threads', '1', '-c:v', 'rawvideo', '-f', 'rawvideo', 'pipe:1', '-map', '[s]', '-threads', '1', '-c:v', 'rawvideo', '-f', 'rawvideo', f'pipe:{write_fd}']
    started = time.monotonic()
    child = subprocess.Popen(base, stdout=subprocess.PIPE, stderr=subprocess.PIPE, pass_fds=(write_fd,))
    os.close(write_fd)
    received = 0
    first = None
    # Model one bounded retained sink value; independent fanout does not wait.
    latest = None
    try:
        os.set_blocking(child.stdout.fileno(), False)
        deadline = started + 2.0
        while time.monotonic() < deadline:
            ready, _, _ = select.select([child.stdout], [], [], min(.05, max(0, deadline-time.monotonic())))
            if not ready:
                continue
            data = os.read(child.stdout.fileno(), 65536)
            if not data:
                break
            received += len(data)
            first = first or (time.monotonic()-started)*1000
            if not fused:
                latest = data  # one bounded current chunk, independent slow sink
        outcome = dict(fused=fused, complete_preview_frames=received//(160*120*3//2), first_byte_ms=first, observation_ms=(time.monotonic()-started)*1000, command=base)
    finally:
        if child.poll() is None:
            child.kill()
        child.wait(timeout=2)
        outcome['stderr'] = child.stderr.read().decode()
        child.stdout.close(); child.stderr.close(); os.close(read_fd)
    return outcome


def main():
    with tempfile.TemporaryDirectory(prefix='blent-camera-fusion-') as temporary:
        root=Path(temporary)
        decode.source(root,160,120,300)
        rows=[trial(root/'160x120.h264',mode) for mode in [False,True]]
        assert rows[0]['complete_preview_frames']==300, rows
        assert rows[1]['complete_preview_frames']<300, rows
        print(json.dumps(rows,indent=2))


if __name__ == '__main__':
    main()
