# USB preference contract and Android controls (T1098/T1099)

The portable contract and Android storage/controls are implemented. Host option
parity and joined configurable-capacity acceptance remain T898. Hosted SDK tests
do not establish new physical device acceptance; application acceptance remains
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
native coverage and source complexity evidence accompany T1098's completion;
interface readiness is not completion of T1099/T898.

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
