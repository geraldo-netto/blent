# Display activity signal contract

Status: independent Rust projection/schema and Kotlin schema validation implemented
under T1055. Host control publication remains T1056, Android decoder consumption
T1057 and wake ownership T1058. No peer resource release is claimed from this
contract alone; joined Android acceptance remains T1011 and physical acceptance T914.

The original authenticated control connection owns the signal sequence. A hint
carries no credential, consent, capture permission or native retirement proof.
An adapter must validate the actual original controller/attachment before sending
or applying it. Equal counters on another connection have no shared identity.
A replacement connection installs a fresh owner and invalidates old callbacks.

The JSON object has exactly four members:

| Member | Contract |
| --- | --- |
| `version` | Integer 1. |
| `revision` | Integer 1 through 9,223,372,036,854,775,807. |
| `stopped` | Integer 0 through `revision`, retaining interruptions of original observed display work. |
| `active` | Boolean desired eligibility, separate from native acquisition. |

Missing/extra members, unknown versions, strings/floats for integer fields,
out-of-range values, future stop fences and non-Boolean activity are rejected.
The Kotlin boundary validates a parsed JSON object; outer framing, syntax and
message capacity remain the control transport's responsibility. Rust serializes
a canonical object below 128 bytes. The schema is independent of engine, UI,
storage, runtime and OS types; shared vectors define language-neutral value cases.

One projection owns its original desired-work observation, last emitted signal
and terminal-failure bit. The existing independent WorkObservation contract
interprets machine publications, including remembered source stop fences. Initial
unavailable observations publish inactive. Continuous unchanged demand emits
nothing. An original observation invalidated by lock, display-off, suspension,
source loss or source replacement increments the stop fence even if latest demand
has already resumed. A consumer must retire its original decoder when that fence
changes; an Active hint cannot skip actual local retirement or fresh codec/config
and keyframe validation.

Absent observation preserves the existing explicit unsupported-source policy.
Composition must distinguish lack of protocol/native observation support from
an unavailable supported source; T1056 supplies that publication boundary. A
signal never infers shutdown from unplug or commands unrelated camera/audio owners.

Revision and stop counters never wrap. Exhaustion makes the original projection
terminal, denies all subsequent publication and requires its adapter to close the
original control lifetime. Reinitializing it under the same connection is forbidden.
A finite counter is an explicit capacity contract, not permission to reuse identity.

The projection contains two bounded value records and one bit, with no allocator,
callback, queue or native handle. Input publications and returned immutable signals
are value copies. The caller owns their storage and bounds retained views and
decoded control messages; copies retain no native authority. Pure operations finish
synchronously. Native cutoff, quiescence, cleanup sponsorship and actual reuse stay
with the existing original control/decoder/wake owners; serialization and model
checks cannot prove native safety. Permanent tests cover shared valid/invalid
vectors, numeric boundaries, terminal exhaustion and 1,024 five-step transition
traces against an independent stop model.
