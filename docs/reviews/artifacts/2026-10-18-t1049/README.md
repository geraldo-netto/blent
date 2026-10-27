# T1049: reversible renewable display registration suspension

Display registration suspension revokes the old route but retains intent and ownership. Resume waits for exact original cleanup and creates fresh credentials; bounded model traces are not physical acceptance.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Pause synchronously revokes the original route/socket generation and requests
client close. It retains selected attachment intent, configuration and the original
client, and continues polling that client's native/remote retirement while paused.
No retry starts during pause. Resume is a desired-work command: renewal still
requires original retirement, retry deadline and a current attachment guard, and
uses fresh credentials. Terminal close remains irreversible.

Five permanent normal-suite tests cover active/idle/startup suspension, retained
configuration, early resume, held retirement, late Active replies, replacement
authority and terminal cancellation. The bounded test exhausts all 1,024 sequences
of five commands from pause, resume, original client retirement and close. Every
renewal requires all preceding clients retired; final cleanup is checked after
the fixture supplies completion evidence. Four cases reject an API-only no-op
pause/resume control; the raw red log retains its preliminary T1046 labels and
the exact no-op source is retained. Final test/ledger identity is T1049. All 17
registration tests pass after implementation.

Fresh offline native collection runs the full host library suite: 552 pass, zero
fail, three existing benchmark ignores. Every one of the 13 production functions
in `registered_display.rs` independently passes the 80% executable-line gate;
minimum is 84.62% for `Attempt::observe`, all others are 100%. The unchanged native
collector retains fresh exclusive build ownership, source/object hashes, raw LCOV
and per-function results. The earlier sandbox attempt fails existing private
D-Bus/socket fixtures with `Operation not permitted`; its failure log is retained.
The final unrestricted local run includes every test without new exclusions.

The unchanged repository Sonar-rule source audit covers all 45 production/test
functions in the two changed Rust files, maximum 6. It uses cached
tree-sitter-rust 0.24.2 and tree-sitter 0.25.2. Native SonarQube measurement remains
unavailable under T1019. Twelve architecture/contract/portability checks and
offline production accessory-enabled compilation pass. That compilation's two
existing idle ownership/test-observer warnings are recorded separately as T1052.

The serialized Display owner retains at most one original registration client
and its attachment authority. Pause adds one boolean workflow state and no native
allocation, worker, callback or independent cleanup owner. Route revocation fences
old socket use; retirement remains the native client's contract. Held or unknown
cleanup keeps the original client and selected capacity owned. Close/drop request
cleanup without manufacturing completion. Eventual renewal/settlement requires
actual original client retirement and continued owner polling; no elapsed budget
releases an obligation. Replacement guards cannot revoke a newer attachment.

This is the portable registration prerequisite from T1016. It does not yet wire
machine activity into production USB routing. T1050 owns exact native capture and
selector fences; T1051 owns their activity/registration composition; T1016 retains
joined automated acceptance. Physical qualification remains T914. No download,
installation, upload, push, live device, lock or suspend operation occurred.

`scoped_rust.py` uses unchanged repository counters/inventory with the cached Rust
grammar extracted and compiled under `/tmp/blent-t1049-evidence/parsers`. Its
`native-final` argument runs the full library suite into a new exclusively owned
target. Source snapshots deliberately reject any source change during collection.

</details>
