# USB capacity/deadline controls (T1098/T1099/T898)

USB settings apply to the next connection and never grant consent or restart active media. The report explains Android persistence, host startup options, shared limits and platform-specific deadline meanings. Hosted storage and joined software tests preserve ownership through cancellation; actual device restarts and physical media acceptance are separate evidence.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

The portable Android preference contract, storage/controls and Linux broker
capacity/deadline controls have joined software acceptance. Hosted tests do not
establish new physical device acceptance; application acceptance remains
T846, with older native provider/accessory qualification still under T863.

Open Settings (⚙), then **USB capacity and deadlines**. Edit values and choose
**Save USB settings**. **Use defaults** changes the editable draft; save it to
replace the stored record. **Reload saved settings** discards unsaved edits only
when reading succeeds. Read errors leave current fields unchanged; a new dialog
starts with defaults before loading. Closing the dialog does not save a draft.

Manual Connect and accessory attachment load the same saved snapshot, then
recheck that another connection has not already started. Saving preferences never
restarts an active worker, erases saved trust or grants camera/microphone consent.

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
native coverage and source complexity evidence accompany T1098/T1099. The shared
T898 fixture now checks corresponding host controls independently of their implementations.

## Storage and start ownership

The Android adapter stores one bounded numeric record in the private
`usb_connection` preferences namespace. It retains only application context.
The application owns one store and admits one original storage API call at a
time, rejecting competing calls before queueing native work. Cancellation of a
request keeps its exact slot until that API call actually returns. The Android
framework owns its internal preference file/writer resources; the store does
not claim an independent native file-descriptor fence or expose those resources.

A successful synchronous publication confirms the save. A false/failed write
may already have changed a backend cache: the store therefore rejects connection
reads until an explicit successful save resolves the unconfirmed publication.
Absent settings select defaults; corrupt settings remain visible errors and are
not silently replaced. Backend error presentation is bounded to 256 characters.
No current worker is replaced because storage is busy, corrupt or unconfirmed.

Each Activity's start controller retains one exact job/identity through actual
completion. It rechecks cancellation and current connection authority after
loading. Closing or losing the original Activity cannot later start its USB
worker from an old result. Editable forms retain eleven bounded strings and own
no active connection. Original API completion is required to release a storage
slot; a configured connection cleanup timeout is not storage retirement proof.
Eventual storage release assumes the framework call eventually returns; a stuck
call keeps its single funded slot without admitting replacements.

The settings dialog fixes its content width independently of platform intrinsic
width probes and keeps actions outside its scroll area. The same small-window
regression that reproduced a two-state width/layout loop passes on hosted SDKs
27 and 34 after that change. No idling deadline, heap limit or assertion was
weakened. [T1099 evidence](artifacts/2026-10-19-t1099/README.md) records the exact
hosted test, ownership-model and per-function coverage scope. Physical storage
reload after application restart remains part of T846 application acceptance.

## Host mapping and application scope

The independent `testdata/connection-settings-v1.tsv` fixes nine corresponding
user settings. Rust exercises actual CLI parsing and the backend validator;
Android exercises typed preference parsing, rejection, encoding and reopening
private storage on hosted SDKs 27/34. The TLS row has the stage distinction below.

| Android field | Linux broker option | Default | Inclusive range |
| --- | --- | --- | --- |
| `channel_bytes` | `--channel-bytes` | 262144 | 4096–1048576 |
| `queued_messages` | `--queued-messages` | 64 | 1–1024 |
| `quantum` | `--quantum` | 65536 | 1–65536 |
| `resource_timeout_ms` | `--resource-timeout-ms` | 120000 | 1–600000 |
| `io_timeout_ms` | `--io-timeout-ms` | 5000 | 1–60000 |
| `retirement_timeout_ms` | `--retirement-timeout-ms` | 5000 | 1–60000 |
| `approval_timeout_ms` | `--approval-ms` | 120000 | 1–600000 |
| `native_timeout_ms` | `--tls-timeout-ms` | 10000 | 1–60000 |
| `trusted_peers` | `--trust-peers` | 64 | 1–1024 |

Host options apply when its broker starts. Each link retains that broker's original
policy; parsing future arguments does not reconfigure an existing broker. Host
changes use normal broker retirement and an explicit later startup. Android saves
persistently and applies the saved snapshot to the next connection worker.
Neither saving Android settings nor preparing future host arguments restarts
active media. No automatic live service/application restart is performed here.

Android's native budget covers its opening, discovery, TLS and window stages.
The host TLS knob covers authenticated TLS, peer proof and preface as one deadline.
Its discovery/controller setup uses `--setup-ms` (60000 by default, 1–60000),
READY uses the approval budget, and window exchange uses the I/O budget. Smaller
underlying I/O limits can still win; for example Android's TLS adapter retains its
per-read socket bound while the supervisor enforces the selected absolute phase
budget. Settings are limits, not promises that every operation lasts that long.

Inventory is also platform-specific: Android `accessory_capacity` defaults to 32
(1–1024); host `--devices` defaults to 64 (1–512). These bound inventory metadata,
not simultaneous tablet ownership. Android `fallback_ms` controls its known-idle
lost-hint recovery; the host uses asynchronous Tokio notifications/timers and a
20 ms resource-maintenance tick, without that same actor fallback option.

Host-only capacities include `--clients` (16, range 1–1024), `--links` (4, 1–64),
`--registrations` (64, 1–1024) and `--block-bytes` (65536, 1–65536). The startup-only
`BLENT_USB_AUTHORITY_WORKERS` environment setting defaults to 1 (1–4096), shared
across broker attempts in that process. Higher capacities permit more retained
work/memory; larger blocks favor throughput over interleaving. All sessions still
have the protocol's four independent resource slots. None of these capacities
waives consent, trust, negotiated windows, native retirement or quarantine.

Native host regressions retain unrelated camera protocol traffic when another
resource expires, retain original identity/native admission after cancellation,
and separate delayed approval from selected TLS expiry. Android integration tests
retain the current worker while saving future settings, including failed and
unconfirmed publication. The shared contract rejects boundary drift independently
in both languages. These are software acceptance results, with production-method
counters in the corresponding implementation artifacts. Physical media, saved
settings across actual app restarts and unsupported platform qualification remain
T846/T863 and their dependencies; no battery/performance gain is claimed here.

</details>
