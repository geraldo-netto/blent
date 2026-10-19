# T1098: portable USB preference editing

Parent revision `ad5e6dd9`. The independent contract is documented in
`docs/reviews/2026-10-19-usb-preferences.md`. Eleven numeric fields delegate
validation to existing typed worker/session/I/O/window contracts. A fixed field
schema uses focused read/replace strategies, keeping platform storage and UI
outside the domain. No native API, permission, trust mutation or active-worker
change is introduced. Android integration remains T1099; host parity and joined
acceptance remain T898.

Six permanent normal-suite tests cover independent defaults/representation,
all field min/max boundaries, simultaneous edits, 64 seeded oversized values
per field, decimal/Unicode/sign/overflow rejection, complete/unknown field sets,
retained-input isolation, duplicate/invalid persistence, 64 oversized records,
field reordering and leading-zero canonicalization. The dangerous integer-wrap
control uses `2^32 + minimum`, which would become an accepted Int without the
preconversion bound. Two deliberate faulty guards fail the two targeted tests;
`fault.py` restores the exact original source with a fresh timestamp in `finally`.
Those red controls are retained, along with final green tests.

Fresh clean native run: **18 tests, zero failures/errors/skips**. The strict
authored inventory has 40 entries; inline bodies are measured in their enclosing
method as required. All **31 mapped native method/initializer entries have 100%
executable-line coverage**, individually. Original XML/exec, class/source hashes,
inventory, per-function report and test XML preserve the scope. No exclusions
or threshold changes. Twelve architecture checks pass. Kotlin PSI 1.9.20 source
estimates cover 60 functions/callbacks, maximum 3; these are not native SonarQube
evidence, which remains missing under T1019.

## Lifetime and bounds

The caller owns immutable specifications and returned maps/text. Parsing borrows
a caller-serialized map only during the synchronous call and retains no input
map or mutable view. There are exactly eleven admitted fields, at most seven
ASCII digits per numeric input and 512 characters per encoded record. The
seven-digit check precedes Long/Int conversion. Decoding bounds materialization
and line splitting before field admission; unknown/missing/duplicate data fails
without publishing a partial configuration. Enum strategies are a fixed
class-lifetime collection, with no dynamic registration.

Intermediate specifications, maps and strings are managed values bounded by
the eleven-field workflow. Success hands the immutable snapshot to the caller;
rejection leaves only unreachable local intermediates for normal managed
collection. No task, native handle, borrowed buffer or release/reuse boundary
is added. Active resource ownership, original completion/fence requirements and
native cleanup remain unchanged. There is no hardware or physical acceptance
claim; all tools and verification ran offline.
