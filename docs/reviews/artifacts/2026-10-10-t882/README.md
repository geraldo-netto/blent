# T882 Android tablet resource session

The tablet resource session keeps bounded loans and sibling progress through revoke and generation replacement. Ring aliasing, publication and cleanup-event regressions passed; native service/media acceptance was separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Independent ticket/approval/resource-lease contracts and the portable tablet
session now compose the existing management, resource policy and multiplexer.
Five bounded receive rings expose exact consumer loans; outbound admission
transfers array ownership. Resource termination preserves sibling progress and
requires actual native/transport retirement before replacement. ACK publication
and replacement Offers remain ordered across asynchronous flush callbacks.

Offline validation, using cached tools/dependencies:

- 166 ordinary connection tests pass, including a 512-round seeded duplex model
  spanning all five lanes, ring wraparound, invalid bounds and exact receipts.
- Eight maintained real Rust/Android TLS interoperability tests pass. The new
  API27/34 scenario connects the real Rust host Session to TabletSession, admits
  display/control, checks exact duplex bytes, revokes and retires the resource,
  then repeats with generation two over the same authenticated connection.
- All 101 inventoried production functions pass the per-function coverage gate.
  Raw JaCoCo counters cover all 210 executable JVM methods in scope at 100%,
  including constructors/getters not inventoried by the current gate (T848).
- All 278 reviewed production/test/fixture functions have CC≤9.

Permanent red→green regressions cover four draft defects: self-aliased ring
wraparound corruption, expired Commit missing host Revoke notification, invalid
exact writer completion leaving partially advanced protocol usable, and event
backlog consuming the space required for four resources' cleanup notifications.
Other retained tests cover the ACK receipt versus flush-completion race, late
Commit drain, permission rejection, stale leases and receipts, malformed records,
consumer stalls, and disconnect with outstanding native buffers.

The runtime uses fixed resource/readiness arrays and avoids temporary resource
lists on per-frame progress. Receive consumers borrow contiguous existing ring
storage; incoming DATA still copies into the ring. These are implementation
properties, not measured throughput or latency improvements.

Evidence includes exact source hashes, pre-run inventory, native counters,
passing test logs, complexity results and each failing regression run. Production
Android native service/listeners remain T854; production media adoption and
physical debugging-off acceptance remain T855/T857–T860. JVM fixtures do not
establish physical provider behavior or native syscall/process retirement.
No downloads, live-device operations or external dependency edits occurred.

</details>
