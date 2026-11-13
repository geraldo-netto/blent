# T1391 — automatic remembered-peer USB initiation

The permanent Linux server startup regression fails before and passes after the
fix. The broker selection passes 247/247 tests. All 12 changed production
functions measure 100% executable-line coverage and complexity at most 6 with
cargo-llvm-cov 0.8.7 and the project syn 2.0.119 frontend.

A running broker with saved peers attempts available candidates without a GUI
Connect command. Physical keys only schedule work: unchanged pins still require
fresh TLS, and changed pins remain Pending. First pairing still needs approval.
ADB-reserved and unknown-availability devices are excluded. One action is admitted
per broker tick, bounded by existing link capacity; failures retry no sooner than
two seconds. Live and retiring original links both reserve their physical key.

Disconnect, denial and revocation suppress the current attachment until confirmed
absence/replug or explicit manual Connect. Unknown inventory cannot clear this
cutoff. The hint map holds at most inventory capacity plus retained link keys;
no native resource is owned by it. Existing Handle/Completion owners preserve
original task/helper fences. Shutdown draining never initiates automatic work.
Tests cover startup, replug, delayed retirement, capacity failure, retry boundaries,
changed fingerprints, explicit cutoff, unknown inventory and invalid times.

These are domain and Linux fake-helper observations, not physical USB acceptance.
Android attachment-overlap handling is T1392; no-new-attachment restart recovery
is tracked separately. The current machine's missing kernel driver remains T1389.
