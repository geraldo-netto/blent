# Hosted tunnel handoff measurement

Run on Linux with the configured Android SDK, cached dependencies and a JDK with
thread CPU counters. This uses private loopback sockets and public test identities;
it never contacts a tablet or an installed Blent process.

```sh
python3 scripts/benchmarks/android-tunnel/run.py /tmp/blent-tunnel-measurement
python3 scripts/benchmarks/android-tunnel/analyze.py /tmp/blent-tunnel-measurement/results.json
```

The output directory must be new. The runner records source hashes and runtime
identity, checks unchanged inputs afterward, and retains incomplete output on
failure. Each completed trial is published atomically. There is no whole-run kill
deadline: 64 finite trials each send 60 frames. Sixteen trials warm up the JVM;
48 measured trials provide three alternating pairs per payload/rate combination.
Socket setup/read timeouts are five-second private protocol deadlines.

The baseline reuses the current ConnectionFrameWriter and StreamConnectionIo:
T1759 writes the borrowed header/payload slices and flushes in one native transaction.
Historical T1755 measurements used three separate completions; their recorded
source hashes/results remain historical and are not the current baseline.
The candidate copies header/body into one reusable 65,576-byte staging array and
flushes on the same native worker as each write. Each slice retains the 65,536-byte limit. A frame transaction admits at most
65,576 bytes across two slices; a full payload still requires two candidate writes.
The same native worker owner performs reads, writes and flushes, and exact request
identity is checked before accepting every completion. The peer validates every
header and payload byte and acknowledges each frame; only one frame is outstanding.

A layered socket observes actual encrypted record headers without retaining their
payload. All trials require TLS 1.3 and report the negotiated cipher. CPU counters
cover the driver and three native I/O workers, excluding the peer. Driver poll
calls, semaphore wait calls and observed ready hints are recorded separately;
these are not kernel wakeup counts. Round-trip timing includes payload preparation,
worker handoffs and peer validation; it is not display latency or pipelined throughput.
The driver reproduces write handoffs, not the full session reactor/Android IPC path.

Input and output streams belong to StreamConnectionIo until all three workers have
exited. Trial cleanup closes the original layered/raw sockets and listener, releases
the peer's final observation wait and joins its exact thread. A failed startup closes
all already-admitted native resources before unwinding. The socket adapter closes
both delegated and inherited Socket state. The normal tool suite retains fragmented
record, invalid-range, record-bound, output-failure and descriptor-identity tests.
The missing-base-close control fails that identity test with one retained descriptor;
the failed test's owned JVM exit contains the deliberate faulty control.

The shared hosted NativeCounters adapter also serves the existing idle benchmark;
its opt-in source set and provenance collector include that shared file. Production
application source sets and dependency graphs are unchanged. No native Android,
USB, battery, full-pipeline throughput or hardware-support claim follows from this
hosted experiment. Three paired trials yield local point estimates, not statistical
confidence or a universal optimization threshold.
