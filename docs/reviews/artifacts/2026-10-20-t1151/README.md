# T1151 — private-process saved-trust maintenance

Saved-trust maintenance runs in the private connection process with bounded requests/pages and read-only identity browsing. Hosted ownership tests passed; Android provider isolation and a separate lint failure remained unqualified.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

This implements the internal adapter for T897. Offline UI and main-process
connection/maintenance arbitration remain T1152; no new UI support or physical
acceptance is claimed here. The existing nonexported `:connection` service admits
one connection or maintenance START in its lifetime. Valid second maintenance
requests, or connection requests while maintenance owns the service, reject
before another native factory runs, including during original retirement.

Maintenance uses private IPC command 5 and metadata event 14; existing tags and
the host wire protocol remain unchanged. Requests are two-byte list or 142-byte
conditional forget records. Pages contain at most 48 peers within the existing
4096-byte packet bound. A 1024-peer snapshot takes 22 pages plus Ended, below the
32-event queue. Counts, identity, revision, order, offsets, booleans, truncation
and trailing bytes are checked. The original client generation filters callbacks;
each operation rejects events of the other operation kind.

Browsing reads only the existing AndroidKeyStore public certificate. It never
generates/repairs keys, requests a private entry, signs, opens USB or grants media.
Shared bounded public identity fingerprinting is reused by setup, verified TLS
and browsing; fingerprint metadata alone grants no proof. No public identity plus
no persisted trust yields an empty result. Existing trust without its matching
identity, interrupted publication, corrupt records or provider failure preserve
storage and fail visibly. Forget requires the displayed local identity, revision
and peer; stale actions fail before mutation. Existing publication poisoning and
revocation rules remain. A cancellation concurrent with durable mutation cannot
undo a committed forget; later requests must obtain a fresh snapshot.

The retained private IO job owns acquisition, trust access, publication and close.
Factory ownership is uncertain until an original access object returns; failed
creation or close requires original process exit. Cancellation cuts off further
publication; every returned access closes in finally. Native stores use T1156's
bounded admission before a second lock descriptor opens. There is one mutable
trust owner; pages and selections are bounded immutable copies, never authority.
Callers must retain complete results through successful operation and original
retirement before publishing a ready view; page arrival alone is insufficient.

Maintenance observation has the existing client's start budget (default 5000 ms),
separate from original native retirement. Default graceful retirement allows
5000 ms plus 1000 ms timer slack, followed by the existing exit wait. Deadline,
Ended and framework disconnect are never release evidence: the original START,
STOP/EXIT/cleanup jobs must return/join and the original Binder must die before
unbind/reuse. A failed retirement retains original binding/jobs for later retry.
Process exit is the enclosing native-access fence for unknown obligations;
conditional eventual cleanup requires actual original return or process death.

The sealed permanent deadline regression fails before observation is bounded and
passes unchanged afterward. Other retained tests cover read-only default provider
access, private service exclusion in both directions, stale callbacks/actions,
native storage ownership through cancellation, storage/reporter/close failures,
request/page boundaries and blocked START/STOP after Binder death. Hosted Binder
death and synthetic KeyStore fixtures do not establish Android device/provider
isolation or physical media acceptance (T863/T855/T858).

All 1033 connection tests pass without failures, errors or skips. JaCoCo 0.8.15
passes all 270 scoped production functions/native variants individually at least
80%; raw counters, source manifest and class hashes are retained here.
Cached Kotlin 1.9.20 PSI reports 639 production/test entries, maximum 8; this is
not native SonarQube evidence (T1019). `assembleDebug` succeeds. `lintDebug`
fails on the existing pre-33 USB receiver registration diagnostic, tracked under
T1158; all other fresh advisories are recorded under T1159–T1166. The raw lint
report is retained, and no full lint pass is claimed. No live app/device, installation, network
transfer or push occurred. Raw local evidence:
`/backups/disk2/blent-work/2026-10-19-open-items/t1151`.

</details>
