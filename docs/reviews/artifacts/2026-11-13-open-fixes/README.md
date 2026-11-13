# Open findings and reconnect fixes — 2026-11-13

The six original Open findings and five additional source/test findings each
have one local fix commit; their resolved TODO rows are removed. Existing
blocked/declined work remains. T1389 retains the newly confirmed host driver
blocker and prepared administrator/reboot recovery steps.

Per-item evidence retains failing-before/passing-after permanent regressions,
function coverage and complexity. T1384 corrects a stale test oracle; T1385 and
T1388 correct current documentation. Existing opt-out, ownership and media-consent
assertions remain. T1394 adds fixture-loss settlement for its synthetic creator.
T1392/T1393 Android evidence is in the adjacent 2026-11-13-reconnect directory.

The final check measures 29,053 functions with zero complexity violations.
Workspace Clippy with warnings denied, Rust formatting, default Windows GNU
cross-compilation and host accessory-enabled binary builds pass. Windows native
execution remains unavailable; optional accessory-feature Windows compilation
was not established because its cross C compiler is unavailable. The feature's
Linux build is separately verified. No Windows runtime claim follows from checks.
The final Android selection passes 58/58 registered cases in ten suites and builds
a debug APK. Test selections are separate invocations, not a combined unique-case
count; verification.json records their scopes.

Read-only service evidence confirms current ADB token delivery, Android launch,
input metadata and a video socket. Capture fails because EVDI is installed only
for the previous kernel. Direct USB also lacked host initiation and Android
reattach/retry handling; those code paths now retain fresh proof, exact cleanup,
changed-fingerprint approval and explicit user cutoff. Fixtures establish bounded
shared/native-adapter behavior, not physical display acceptance. Android automatic
retry belongs to the Activity; process-death recovery is not claimed.

Builds were not installed. The running desktop/Android artifacts predate these
changes. No package installation, device mutation, service/display restart,
reboot, push or publication was performed. The owner requested that native
verification remain blocked for now. Native frame delivery remains blocked
in T1389 until administrator setup and an owner-controlled reboot can be completed.
