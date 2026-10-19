# T1103: configured original-link resource admission timeout

Parent revision `c1c21249`. Host link dispatch previously passed a fixed
120000 ms to every resource registration. `--resource-timeout-ms` now selects
1–600000 ms, default 120000, matching the portable resource authority's existing
admission bounds and Android's corresponding setting.

The independent `AdmissionTimeout` value owns validation and the host default.
It has a private scalar, explicit checked construction, an immutable accessor and
no transport/backend dependency. Existing `Resources::insert` uses this same
validator before checked absolute-deadline addition. A relative timeout cannot
wrap a monotonic deadline or publish a partially admitted slot.

Raw CLI settings validate before native admission/filesystem effects. One backend
adapter maps each snapshot into its link's narrow settings, retaining the original
identity guard and monotonic origin. Native creation starts only afterwards.
Each link's media workflow carries the typed timeout by value into dispatch.
Dispatch now borrows the session view it needs, independently of the native
Connection owner. Saving/selecting a future snapshot cannot mutate an old link.

## Permanent regressions and faulty controls

The unchanged CLI regression fails before the option exists, then passes after
the change. Boundary/malformed/signed/overflow inputs exercise both parsing and
actual startup rejection before runtime/state directories appear. The portable
contract covers boundaries, 4096 deterministic valid/invalid inputs, failed
absolute additions with unchanged next request identity, and late peer records.
Host late Prepared remains a coordination record returning `Ok(false)`; Commit
is refused and it cannot grant access. This preserves settlement obligations.

Configured dispatch at 1, 37, 120000 and 600000 ms is exercised against an
independent protocol clock. It emits the original registration, remains open
before its deadline, then emits Closing exactly at expiry. An unrelated admitted
camera retains its lease and delivers/acknowledges actual protocol payload bytes
after that expiry. No local connector is polled in this admission-only fixture.
The backend mapping test changes future settings while retaining the original
snapshot and identity, and rejects invalid construction instead of defaulting.

Both deliberate faulty controls are rejected by these normal tests: replacing
actual dispatch's selected timeout with 120000, and replacing backend mapping's
selected value with the default. Sources are restored in `finally` before fresh
native collection. Two initial draft-driver failures are retained: the first
omitted the session's explicit clock tick; the second expected an error instead
of the documented late-Prepared `Ok(false)` coordination result. Neither prompted
a production behavior change or weakening of access/expiry assertions.

Fresh offline native coverage includes host/common libraries and actual broker
executable election, signal/restart fixtures. **1265 tests pass** (894 host, 369 common, 2 executable integration); three
existing host opt-in benchmarks remain ignored. All **47 scoped production
functions** pass their individual 80% gate, minimum 83.33%.
The initial gate found `Resources::offer` unmeasured (0/4 lines). Permanent
tablet-offer timeout/overflow/role/identity tests now cover it at 100%, with
no exclusions. All 123 source estimates are at most 9. Exact source/object
attestations, LCOV and individual function results accompany this report. Installed
rustc 1.98.1/cargo-llvm-cov 0.8.7 use an exclusively owned fresh target. Twelve
architecture checks pass. Cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2 source
estimates are not native SonarQube measurements; T1019 remains unresolved.

## Lifetime and limits

The timeout contract retains no allocation, borrowed view, callback or native
resource. Each link owns one immutable scalar inside its existing admitted task;
there is no new mutable state owner, queue, timer thread or pool. Configuration
validation rejects before publishing a view. Partial mapping failure drops its
already moved/copied fields through ordinary ownership, including any identity
reference, while the server retains its original authoritative identity.

Expiration revokes admission and starts existing cleanup; it cannot establish
native retirement or release retained queues, pumps, keys or original admissions.
Existing exact completion/settlement fences remain necessary. An admitted resource
is unaffected by another resource's admission deadline. Domain fixtures own finite
message lanes and buffers; all borrowed read receipts are acknowledged before
scope end and connectors are explicitly disconnected without native creation.

Longer admission timeouts retain pending consent/preparation longer; shorter ones
reject slow admission sooner. Existing native, I/O and retirement budgets remain
independent. Conditional eventual cleanup still requires original native work and
destructors to return and existing reapers to be scheduled. Real native ownership
is covered separately by retained Linux/TLS retirement regressions, not proved by
this scalar policy or hosted camera-data fixture. Physical media acceptance and
unsupported platforms remain unresolved; joined settings acceptance remains T898.
