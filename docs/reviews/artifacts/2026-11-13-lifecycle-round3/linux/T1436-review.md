# T1436 — initial compositor query failure discards observation

The independently requested follow-up after T1429 confirms a separate initial
discovery outcome. An existing private Cinnamon D-Bus owner implements
PowerSaveMode and reports display-off mode3. Its first property query returns
one transient Failed reply. Factory/Following then publish display capability
false and display demand true. Five configured retry intervals later, publication
and epoch/revision are unchanged and the getter has been called exactly once.
A direct query to the same owner succeeds with mode3 while Blent still requests
display. Exact output is `initial-display-failure.log`; public-API probe source
is `initial_display_failure.rs`.

`activity/native.rs::Actor::display` propagates the initial query failure;
`activity.rs::LinuxSource::discover` classifies every display actor creation error
as unsupported and starts the source without that actor. The resulting source
has no display observer to request recovery. This causal source trace establishes
the missing retry; the finite observation alone is not permanent-loss proof.

Severity medium, effort small. Distinguish a transient read failure against a
known native owner from absent/explicitly unsupported observation. Preserve
conservative unknown state and original native retirement until fresh snapshot
recovery; permanent tests must include initial error, later off-state receipt,
prolonged failure/stop and genuinely absent/unsupported controls. T1429 addresses
initial boolean-read failure only and does not resolve this display path.

The probe uses existing library artifacts and the same external rustc dependency
fingerprints/command pattern as `review.md`; run its external binary beneath
`dbus-run-session` with LLVM_PROFILE_FILE under
`/tmp/blent-lifecycle-round3-linux`. No live bus, service, device, display or
configuration is changed. Following retires before the private server closes.
All7 authored functions measure complexity<=3 with the project syn2.0.119 frontend;
`initial-display-complexity.json` retains source/library hashes. Native source
changes between the earlier artifact and this review do not alter the reviewed
display-discovery error path; a permanent current-build regression is required
before fixing it.
