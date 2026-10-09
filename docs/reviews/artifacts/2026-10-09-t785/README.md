# T785 — production encoded-reader capacity

Decision: retain the current 8 KiB `BufReader::new` default. Larger buffers can
reduce reads when complete packets are already queued, but showed no consistent
latency improvement across packet sizes and fragmented producer writes. A 64 KiB
buffer adds 56 KiB per reader (8x the buffer allocation). No runtime change is
justified by these measurements. This closes the evaluation, not a claim that
8 KiB is optimal for every workload.

The retained benchmark invokes the actual `cli_encoder::Packetizer`, hence
`FramedAnnexB` and `IvfPacketizer`, over a real Linux pipe. Each counted source read
executes `File::read`; short reads and total requested bytes are recorded. It
uses a blocking AsyncRead test adapter to isolate buffering/framing from Tokio
readiness scheduling. These are local framing timings, not decoder, USB or
end-to-end latency. Production `read_loop_with_idle` and publication are unchanged.

Three counterbalanced trials, 80 packets each: H.264 framecrc/Annex B and VP9 IVF;
1/32/256 KiB payloads; 8/32/64 KiB reader capacities. Modes: 0 queues continuously;
1 writes one complete packet then waits for consumer acknowledgement; 2 fragments
the first 44 bytes into 3-byte writes and the rest into 3001-byte writes, then
waits for acknowledgement. No artificial wait to fill the reader.

Examples (median of three trials; 80 packets):

| Format / producer / payload | 8 KiB reads / p95 | 32 KiB reads / p95 | 64 KiB reads / p95 |
| --- | ---: | ---: | ---: |
| H.264 / queued / 1 KiB | 11 / 5.55 us | 4 / 3.72 us | 3 / 5.11 us |
| H.264 / one packet / 32 KiB | 160 / 96.11 us | 160 / 75.11 us | 80 / 73.04 us |
| H.264 / fragmented / 256 KiB | 7254 / 501.81 us | 7087 / 561.70 us | 7052 / 577.92 us |
| VP9 / one packet / 256 KiB | 404 / 137.70 us | 403 / 142.62 us | 400 / 171.47 us |
| VP9 / fragmented / 32 KiB | 1205 / 12.86 us | 1140 / 25.71 us | 1161 / 20.19 us |

The 32 KiB packet includes framing, so a 32 KiB buffer still needs another read
in mode 1. At 256 KiB, large reads already bypass an empty small buffer; enlarging
it does not eliminate the producer/pipe's short reads. Requested bytes are not
bytes copied or allocated: a large requested read may return a short chunk.
Full first-packet, p50/p95, syscall, short-read and allocation results are in
`benchmark.json`. Timings include thread scheduling and are intentionally not
presented as statistically significant hardware-wide speedups.

Permanent normal-suite tests cover each capacity, payloads on either side of all
three buffer boundaries and a 256 KiB packet. Ready frames arrive with the writer
held open and no next packet/EOF; subsequent pending reads remain cancellable.
Bounded truncations and zero/oversized IVF lengths still reject. Existing Annex B
invalid-header/checksum and IVF bounds tests remain. No new production functions
or altered production behavior; no new coverage exclusion.

```
cargo test --offline -p blent --bin blent t785
cargo test --offline -p blent --release --bin blent t785_pipe -- --ignored --nocapture
cargo test --offline -p blent --bin blent framed_annex_b::
cargo test --offline -p blent --bin blent ivf::
```
