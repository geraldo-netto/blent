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

Both modes use the current ConnectionFrameWriter and StreamConnectionIo, writing
and flushing in one native transaction. The baseline disables gathering; the
candidate uses the production 16-KiB payload threshold with a reusable 16,424-byte
scratch array. Larger frames borrow their original arrays in both modes. Each
result records its selected gather limit. Historical T1755/T1760 exploration
artifacts preserve their original source hashes and benchmark-only candidates;
those old implementations are not activated by the current runner.

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

For local consumer coordination, select the private plaintext TCP analogue:

```sh
python3 scripts/benchmarks/android-tunnel/run.py /tmp/blent-local-measurement --kind local-write
python3 scripts/benchmarks/android-tunnel/analyze.py /tmp/blent-local-measurement/results.json
```

Here `combined=false` runs the current production LocalChannelConsumer and its
exact ring receipt. `combined=true` is a direct WriteFlushed control on the same
production StreamConnectionIo, bypassing consumer bookkeeping. The native control
holds the original payload through exact write/flush completion; neither mode
copies payloads. Prefix/suffix sentinels and peer byte checks detect wrong ranges.
Both use TCP_NODELAY, private five-second setup/read deadlines and one ACK per
payload. This is Linux JDK TCP, not Android LocalSocket or a full resource pump.
Both modes now submit one native write-and-flush transaction per payload. The
direct control measures remaining consumer bookkeeping; comparison to historical
baselines also changes measurement runs and cannot isolate causality by itself. Later runs always use the
current consumer source, so historical baselines remain artifacts rather than
executable legacy paths. The shared `tls_application_records` field is known zero
for this explicitly plaintext transport, not a TLS measurement.

The local fixture owns one listener, original client socket and peer thread;
the accepted socket stays inside the peer's use scope. MeasuredIo owns the three
native workers and borrows abstract streams through MeasurementConnection. Its
close revokes admission, closes the original stream and joins those exact workers;
the enclosing connection closes the listener and joins the original peer. Failed
construction closes already-admitted originals. Wrong-byte controls require the
peer's explicit mismatch error and confirm the client socket is closed afterward.
The normal tool fixture runs local valid/invalid/fault cases alongside TLS-record
and output-path tests, and checks registration of both executable tasks.

For receive-side read staging, run both peer write policies:

```sh
python3 scripts/benchmarks/android-tunnel/run.py /tmp/blent-read-default --kind tls-read
python3 scripts/benchmarks/android-tunnel/run.py /tmp/blent-read-gather --kind tls-read --peer-gather-bytes 16384
```

Here `combined=false` borrows the production ConnectionFrameReader's exact header
and body destinations directly into StreamConnectionIo. `combined=true` reads into
one benchmark-only 16-KiB array, then copies its retained prefix into those same
production framing loans. A native loan blocks scratch reuse; remaining bytes must
be consumed before another native read. The parser's single completed frame must
be released before subsequent framing. This experiment changes no production read
policy and does not change the independent 16-KiB WebSocket default.

A request token paces each peer frame, giving one outstanding frame and a measured
round trip. The peer uses the production frame writer and selected gather policy,
then synchronously writes its exact slices and flushes. Both reader modes validate
the header, sequence and every payload byte. `extra_copied_bytes` records the added
staging copy; exact reads add none. The shared completion count includes the outgoing
request token (the role previously occupied by the writer benchmark's incoming ACK).
`tls_application_records` measures only outgoing request-token records here, as
explicitly labelled by `record_direction`; it is not an incoming record count.

Read-ahead retains 16 KiB in addition to the production reader's 40-byte header and
64-KiB body arrays, excluding object headers. It copies each received byte once and
keeps at most one native scratch loan and one parsed frame. The same private TLS
connection/native-worker owner handles both directions. Any fixture failure closes
the original streams and joins the exact workers/peer before releasing its storage.
Tests cover foreign/duplicate/invalid completions, prefix fragmentation, guarded
extents, retained-loan reuse rejection, wrong bytes/headers and actual TLS exchange.
The one-outstanding-frame domain does not measure pipelined multi-frame read-ahead,
Android ART/LocalSocket behavior or full-reactor deadlines under native cancellation.

For multiplexer scheduling allocation, use the current production mux:

```sh
python3 scripts/benchmarks/android-tunnel/run.py /tmp/blent-mux-allocation --kind mux-allocation
python3 scripts/benchmarks/android-tunnel/analyze_mux.py /tmp/earlier-mux/results.json /tmp/blent-mux-allocation/results.json
```

This separate finite domain has 14 trials: four warmups and ten measured trials,
five each for idle scheduling and 64-byte data transfers. Every trial executes
200,000 passes. Data mode grants exactly one payload of peer credit, offers the
same original array only after its preceding frame finishes, checks its identity
and length, and finishes the exact reserved frame. Idle mode requires no frame.
The loop owns one mux and one 64-byte payload; initialization is outside timing.
No sockets, device access, processes or coroutine workers occur inside a trial.

Hosted thread counters measure allocations and CPU for the driver; elapsed time
also includes scheduling and collection pauses. Counter sampling has a small fixed
per-trial allocation cost, so tiny residual bytes/pass are not attributed to the
mux. There are no hardware/ART counters here. The analyzer reports medians of five
trials and ratios between separate runs; historical artifacts preserve earlier
source hashes without retaining an executable old implementation. Production
snapshot capacity and exact behavior still require separate ownership/regression
checks; a lower allocation counter is not a leak-freedom proof.
