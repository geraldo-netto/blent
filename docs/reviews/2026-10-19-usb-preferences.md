# USB preference contract (T1098)

The portable editing/persistence contract is implemented. Android storage and
controls remain T1099; host option parity and joined acceptance remain T898.
The application continues to use its existing defaults until those adapters
are integrated. This document does not claim new native device acceptance.

Settings describe one immutable **next-connection** configuration. They neither
change an active worker nor grant trust, USB permission or media consent. The
local device selection is supplied separately by the caller and is not stored
in the preference record. The authoritative validation owners are the existing
`ConnectionWorkerSpec`, `ConnectionSessionLimits`, `ConnectionRunLimits` and
`ConnectionWindow` contracts; the field adapter delegates to them.

| Field | Default | Inclusive range | Tradeoff |
| --- | --- | --- | --- |
| `channel_bytes` | 262144 | 4096–1048576 | Larger channel windows can retain more buffered data; negotiated capacity is no greater than either side's limit. |
| `queued_messages` | 64 | 1–1024 | More queued control/data records consume more memory and can wait longer. |
| `quantum` | 65536 | 1–65536 | Smaller scheduling quanta favor interleaving with more dispatch work. |
| `resource_timeout_ms` | 120000 | 1–600000 | Time allowed for resource admission; longer waits retain pending ownership longer. |
| `io_timeout_ms` | 5000 | 1–60000 | Native I/O progress budget; shorter values can reject slow operations. |
| `retirement_timeout_ms` | 5000 | 1–60000 | Cleanup observation budget; expiry never proves native retirement or permits unsafe replacement. |
| `approval_timeout_ms` | 120000 | 1–600000 | Time for local pairing/permission approval. |
| `native_timeout_ms` | 10000 | 1–60000 | Native opening/setup operation budget. |
| `trusted_peers` | 64 | 1–1024 | Maximum saved trust records; a limit below the saved count rejects connection setup rather than erasing or granting trust. |
| `accessory_capacity` | 32 | 1–1024 | Bound on accessory inventory/admission; higher limits admit more metadata. |
| `fallback_ms` | 100 | 1–1000 | Longer known-idle lost-hint fallback reduces polling but increases recovery latency when a hint is missed. |

Earlier absolute deadlines and smaller asynchronous budgets still win. Unknown
deadline providers and retirement retain their 10 ms maximum polling fallback.
Capacity and timeout settings are not promises of hardware performance or
cleanup success. Existing quarantines, original owners and actual retirement
fences remain authoritative.

The field map must contain exactly these eleven keys. Numeric input is 1–7 ASCII
decimal digits, checked before conversion; no signs, spaces, fractions, exponents,
Unicode digits or integer truncation. Canonical output uses ordinary unsigned
decimal text. Leading zeroes accepted during editing are removed on encoding.
The caller serializes any map edits during parsing; the resulting specification
retains no map or mutable input view.

The private persistence representation is at most 512 characters. Its first line
is `1`; subsequent lines contain `key=value` in the table's fixed order. Decoding
accepts field order independently but requires the complete set with no unknown,
duplicate or missing key, extra line or trailing separator. Parsing constructs
validated immutable intermediate values only; rejection publishes no partial
configuration and performs no native effects. A storage adapter must distinguish
an absent record from a corrupt record and must report failed persistence.

The permanent `ConnectionPreferencesTest` covers independent defaults, all eleven
field boundaries, simultaneous edits, strict/overflow/oversized rejection,
unknown/duplicate/missing fields and retained-input isolation. Deliberate faulty
length and unknown-field guards must fail the invalid-input controls. Per-method
native coverage and source complexity evidence accompany T1098's completion;
interface readiness is not completion of T1099/T898.
