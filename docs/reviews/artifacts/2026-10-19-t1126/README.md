# T1126 original camera creator fences

Camera producers reserve child and feeder observations before native creation,
then bind their exact original completions before handoff/yield. Failed feeder
admission still retains an already created child through actual reap/dependent
cleanup. Device locks stay with OwnedChild; task completion independently covers
future destruction and join. Decoder admission has one slot, so unretired original
decoders cannot admit native replacements. USB validates the request, reserves
its original slot, invokes its synchronous typed listener creator, then binds the
actual bridge completion. No borrowed listener reference escapes that callback.
Direct native outcomes also wait for all original reported obligations.

The table observes proof, not ownership. Native owners/reapers retain resources
on success, rejection, partial failure, cancellation and exhausted work admission.
Cutoff and observation funding precede effects; callbacks cannot admit new work
after their original permission is sealed. Reuse requires exact completion, not
a timer, logical result or descriptor number. Existing bounded graceful retirement
may request abort; uncertain resources stay funded until actual cleanup. Eventual
progress assumes native owners settle and their observers receive execution.
Containing-process death remains a separate unresolved fence under T1137; these
in-process observations are not a crash-surviving ownership proof.

Permanent normal-suite regressions reproduce producer admission/cancellation,
decoder replacement/missing proof, USB missing admission/proof, direct outcome
escape and pre-admission listener creation. Red logs retain each before-fix stage;
the listener-order test follows behavior-preserving creator extraction, before
moving creation behind admission. All ten T1126 tests pass, including partial
feeder failure, failed listener creation and 38 invalid credential byte/length
cases that never invoke the listener creator. No assertion is weakened. Private
mount/device fixtures reacquire their exact original device locks before asserting
results, including the deliberate failing source. Native process/resource cleanup
is distinct from simulated USB selection and unavailable physical acceptance.

Fresh offline, source/compiled-object-attested LLVM collections pass:
- Default: 628 tests; three existing opt-in timing benchmarks ignored.
- accessory-transport: 919 tests; the same three benchmarks ignored.
- Focused camera cohort: 76 tests, no failures/ignores.

The combined gate covers every production function in camera.rs, decoder.rs,
usb.rs, camera_control.rs and camera_control/native.rs: 46/46 individually pass
>=80% executable lines (minimum 84.62%), with no new exclusions. Pinned Rust
source audit covers all 94 affected production/test/helper functions, maximum7;
this is not native SonarQube evidence (T1019). Raw counters/manifests/objects:
`/backups/disk2/blent-work/2026-10-19-open-items/t1126/native-final-{default,usb}`.
The earlier native-default collection predates the listener creator change and
is not used as final evidence.

Clippy remains red on existing portable-connection and host issues, recorded
independently as T1130/T1132–T1134. The EVDI-specific capture-control lint is
explicitly deferred under T1131. No suppressions or dependency downloads were
used. Cargo workspace formatting passes; the Linux-supervisor include formatting
scope remains deferred under T1128. The CLI Ctrl-C registration-error gap present in this historical
collection is resolved separately under [T1129](../2026-10-19-t1129/README.md). T1090 ADB ownership remains blocked by owner
choice. Parent T1089 and machine-availability integration remain unresolved.
No Windows/EVDI changes, live camera/tablet actions, installation or upload.
