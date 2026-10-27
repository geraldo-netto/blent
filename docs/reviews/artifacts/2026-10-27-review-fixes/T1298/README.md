# T1298: one current audio wire contract

The host and Android accept only `BLAUREQ2` / `BLAUD002`. Every PCM frame
contains the 24-byte native-counter prefix, including all-zero counters when
native time is unavailable. Removing format-selection flags does not require
native timestamp availability. Both directions retain their independent owners,
authentication, consent, deadlines, fixed PCM sizes and retirement boundaries.
The private Linux helper takes exactly queue target, owned node name and direction;
its IPC blocks always contain the discontinuity byte, counter prefix and PCM.
Deploy matching host and Android builds; no downgrade or compatibility path remains.

## Regressions

Before implementation, the three Rust T1298 regressions failed: old requests and
1921-byte native speaker frames were accepted, and default grants emitted v1.
Android's default-request and old-grant rejection regressions failed; its existing
v2 missing-counter framing passed. The same permanent regressions now exercise
the current contract, including rejection without receiver sequence advancement.

Existing coverage retains native counters present/absent, stereo ordering,
PCM extrema, invalid flags/counters, bounded truncation and oversized input,
sequence gaps, replay rejection, atomic buffer reuse, deadline and partial-I/O
failure, original connection loss, independent duplex stop and held retirement.
Fake native helpers now select counter availability explicitly through fixture
state; their framing never changes. TLS tests still demand the original nonzero
native-counter oracle. No timing or ownership assertion was removed.

## Ownership and bounds

`AudioSession` owns the grant generation and admission; its original connection
registration and `OwnedChild` retain native obligations through errors,
cancellation, caller loss and delayed dependent retirement. Removing the format
boolean does not change those handoffs, fences or cleanup sponsors. Tests retain
held native/remote proofs and reject premature replacement admission.

The encoder borrows live samples, validates the complete counter and sample count
before mutating its caller-owned output, and emits at most 1972 bytes. The receiver
borrows a validated frame and commits sequence state only after complete prefix
validation; views cannot outlive their input. Android's reader owns fixed PCM
storage and returns its decoded block within the live connection worker. Native
workers retain their `Arc` queue references until worker exit; process ownership
remains held until actual helper/dependent completion. The native clock monitor
is enabled for microphone direction; production Android already selected this
counter path before removal. Speaker clock extraction and callback pointer
lifetimes are unchanged.

Conservation: each admitted stream retains its original child/connection owner
until exact completion evidence; logical Stop alone cannot release that owner.
Bounded framing cannot admit truncated PCM or commit invalid counter state.
Conditional cleanup still requires actual worker/process/remote completion;
a blocked external obligation remains owned. Private PipeWire and hosted tests
exercise these boundaries but do not establish universal leak freedom or physical
Android/USB/device acceptance. T1258 and other existing native acceptance blockers
remain explicit in `TODO.md`.

## Evidence

Compressed logs retain failing-before and passing-after results. `metrics.json`
records per-function native coverage, changed-file complexity, source hashes and
local tool versions. The source manifests fingerprint production and Rust test
inputs. One refresh updated only the staging fixture oracle; the comparison
artifact verifies identical production/Android hashes and function inventory
through the Android run. LLVM counters were cleared and the full native
collection restarted after that fixture update. The isolated LLVM target collection and the ordinary Android JaCoCo task avoid
stale caller/counter reuse. Android uses the committed T1302 worker budget and
cached offline SDK jars, with no heap/fork overrides. No network transfer or
physical device operation was performed.

The final native collection also covers the default selection controller, which
all-features excludes under `inproc-encoder`. After relocating the audio test
module and the separate lint fixes, counters were cleared and all host/common
unit binaries plus audio, mapping and changed common integration targets rerun.
`verification.json` distinguishes that run, the earlier broad green run and the
full-workspace staging assertion that was corrected. Native-helper Clippy
follow-ups T1308/T1309 are tracked independently.
