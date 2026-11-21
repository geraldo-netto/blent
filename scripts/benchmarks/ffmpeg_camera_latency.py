"""Bounded, serial picture admission to FFmpeg stdout; no tablet/camera access.

One original subprocess and pipe pair per observation. Each picture is followed
by the production AUD delimiter, then awaited before admitting its successor.
Three seconds is the picture-output protocol deadline, not a benchmark runtime
cap. Failure closes admission, kills and waits for the original child. No
reader/writer task outlives that wait. Encoded input is capped at 64 MiB. Only one
picture is admitted at a time; readexactly can temporarily copy its staging frame
into the returned frame (each <=1920*1080*3/2 bytes). Asyncio input staging may
copy the one admitted packet. Completed frames are discarded after hashing.
"""
import asyncio
import hashlib
import resource
import tempfile
import time


def access_units(clip):
    with clip.open('rb') as source:
        data = source.read(64 * 1024 * 1024 + 1)
    if len(data) > 64 * 1024 * 1024:
        raise ValueError('encoded sample exceeds 64 MiB')
    # libx264 aud=1 emits the same four-byte start code for each AUD.
    starts = []
    offset = data.find(b'\0\0\0\1\x09')
    while offset >= 0:
        starts.append(offset)
        offset = data.find(b'\0\0\0\1\x09', offset + 5)
    if not starts or starts[0] != 0:
        raise ValueError('generated sample lacks initial AUD')
    starts.append(len(data))
    return data, starts


async def picture(child, packet, frame_bytes):
    started = time.perf_counter()
    child.stdin.write(packet)
    child.stdin.write(b'\0\0\0\1\x09\xf0')
    await child.stdin.drain()
    frame = await child.stdout.readexactly(frame_bytes)
    return frame, (time.perf_counter() - started) * 1000


async def pictures(child, data, starts, frame_bytes):
    digest = hashlib.sha256()
    timings = []
    epoch = time.perf_counter()
    for index in range(len(starts) - 1):
        await asyncio.sleep(max(0, epoch + index / 30 - time.perf_counter()))
        packet = memoryview(data)[starts[index]:starts[index + 1]]
        frame, elapsed = await asyncio.wait_for(picture(child, packet, frame_bytes), 3)
        digest.update(frame)
        timings.append(elapsed)
        del frame, packet
    child.stdin.close()
    await child.stdin.wait_closed()
    if await child.stdout.read(1):
        raise RuntimeError('decoder emitted an unexpected extra frame')
    return {'bytes': len(timings) * frame_bytes, 'sha256': digest.hexdigest(),
            'picture_ms': timings, 'reads': len(timings)}


async def run(command, data, starts, frame_bytes, errors):
    child = await asyncio.create_subprocess_exec(
        *command, stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
        stderr=errors, limit=65536)
    try:
        result = await pictures(child, data, starts, frame_bytes)
        code = await child.wait()
        if code:
            raise RuntimeError(f'FFmpeg exited {code}')
        return result
    finally:
        if child.returncode is None:
            child.kill()
        child.stdin.close()
        while await child.stdout.read(65536):
            pass
        await child.wait()


def observe(command, clip, frame_bytes, pipe_bytes):
    if pipe_bytes != 65536:
        raise ValueError('latency probe uses the unmodified pipe capacity')
    data, starts = access_units(clip)
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.perf_counter()
    with tempfile.TemporaryFile() as errors:
        result = asyncio.run(run(command, data, starts, frame_bytes, errors))
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    result.update(wall_ms=(time.perf_counter() - started) * 1000,
                  child_user_ms=(after.ru_utime - before.ru_utime) * 1000,
                  child_system_ms=(after.ru_stime - before.ru_stime) * 1000,
                  requested_bytes=None, effective_bytes=None,
                  child_voluntary_switches=after.ru_nvcsw - before.ru_nvcsw,
                  child_involuntary_switches=after.ru_nivcsw - before.ru_nivcsw)
    return result
