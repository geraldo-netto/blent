# T881 Android framing and window agreement

Portable incremental BLCP negotiation now matches the host contract. Separate
framed reader/writer owners retain exact native array loans, partial offsets and
absolute active-I/O deadlines. No OS, service, TLS authority or media-consent
implementation is embedded in these state machines. Native service retirement
remains T854; resource-session composition remains T882.

Validation ran offline with cached Gradle, SDK and Rust dependencies:

- 125 ordinary connection tests pass, including 15 frame and ten window tests.
- Six maintained Rust/Android TLS interoperability tests pass; the new scenario
  runs on API27/34 JVM fixtures and uses actual Rust broker window negotiation
  and Transport/Mux CREDIT/DATA against Android window/frame owners.
- The normal Rust golden-vector test consumes the same eight valid and 15
  invalid window records as Android.
- All 36 inventoried Kotlin functions pass per-function coverage. Raw JaCoCo
  counters independently cover all 57 executable JVM methods at 100%, including
  constructors and getters missing from the current source inventory (T848).
- All 100 reviewed Kotlin/Rust production and test functions have CC≤9.

Permanent red→green regressions reject duplicate native write/flush loans,
completed-frame retrieval after cancellation, and repeated consumer claims.
Additional tests cover malformed/extreme extents, 4096 window mutations, 512
header mutations, fragmented/coalesced frames, payloads through 64 KiB,
borrowed outbound payload identity, stale callbacks, deadlines, zero progress,
and writer progress while the reader retains a native loan.

`coverage.json` records the exact source scope; `native-methods.json` includes
raw constructor evidence. The source manifest, JaCoCo XML/execution data,
source hashes, complexity results, red logs and passing test logs are retained.
No tests or coverage thresholds were removed or weakened.

No device access, downloads, installation or USB-mode changes occurred.
JVM TLS interoperability does not establish physical Android provider support,
debugging-off production media, blocked-syscall termination or worker exit.
The native supervisor must observe deadlines while calls block and retain
quarantined ownership until actual retirement.
