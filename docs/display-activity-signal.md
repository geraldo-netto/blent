# Display activity signal contract

Status: independent Rust projection/schema and Kotlin schema validation implemented
under T1055; original host control publication implemented under T1056. Android
decoder consumption remains T1057 and wake ownership T1058. No peer resource release is claimed from this
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

## Original host control publication

Portable session startup binds the same read-only machine source to capture and
input before exposing listeners/work. Each accepted controller retains that view;
authentication and the original attachment gate precede any hint. The initial
`connected` response includes `display_activity` only for configured observation.
Later standalone `display_activity` responses carry the same validated Signal.
Configured Pending/Failed/Stopped or lost supported observations report inactive;
absence of a configured source omits the protocol rather than claiming support.
Underlying backend capabilities remain their independent observation contract.

Each original socket owns one projection. The control loop arms a source notice
before every current-state read, including the first turn after greeting or write.
It observes coalesced stop fences and suppresses unchanged hints. Original
controller replacement/attachment retirement cut off pending publication through
existing guards. Projection errors propagate out of the original handler, closing
its socket and controller lease; finite exhaustion is proven in the common
contract tests, without claiming a native full-counter-cycle experiment.

The source notice is shared with other read-only activity views. Closure publishes
an inactive observation once; subsequent closed notices remain pending. There is
no periodic input timer, new source factory, detached publication task or hint
queue. Input admission remains bounded to 16 sockets, each retaining one projection,
one latest source view and at most one pending serialized response/notice. The
wire hint remains below 256 bytes. Original socket/task/attachment owners sponsor
cleanup on errors, cancellation, peer loss and native shutdown; hints do not
release their resources or alter camera/audio consent.
