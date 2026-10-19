# T898: joined configurable USB software acceptance

Implementation prerequisites are complete: Android preferences/storage/controls
T1098/T1099; host authority admission/identity lifetime T1100/T1102; configured
retirement/resource/TLS budgets T1101/T1103/T1104/T1105. This item adds independent
cross-language acceptance and reconciles documentation with verified code.

The shared `testdata/connection-settings-v1.tsv` specifies nine field/CLI mappings,
defaults and inclusive ranges. Rust exercises real CLI parsing and backend
validation, all boundaries, invalid adjacent values and retained snapshot
isolation. Android exercises typed preference parsing, invalid/partial rejection,
canonical persistence round trips and unchanged current-device snapshots.
Hosted Android SDK27/34 tests save every boundary through the actual private
SharedPreferences API, reopen the store, and verify the complete saved record.

Two normal Rust tests and **68 selected Android tests pass**, zero failures,
errors or skips. The Android set includes existing storage cancellation/history,
original-start authority, controls, small-window UI and activity/recreation tests.
No production behavior changes in this acceptance item: **667 tracked production
Rust/Kotlin source files** are byte-verified against the completed implementation
revision. Kotlin source estimates cover 19 test functions, maximum 3; Rust/tool
estimates cover 10 functions, maximum 3. Twelve architecture checks pass. Source
estimates are not native SonarQube evidence; T1019 remains unresolved.

A deliberate fixture fault changes trusted-peer default 64 to 63. Both normal
language suites reject that precise default mismatch. The Android JUnit message
highlights changed digits as `6[3]` versus `6[4]`; the initial log matcher expected
unhighlighted digits and failed after the intended test rejection. Exact numeric
mismatch was then verified after removing formatting brackets. Raw logs/XML,
corrected validation and the byte-verified restored fixture are retained. Both
final suites pass with the correct nine-field fixture.

## Integration and lifetime scope

Corresponding limits, host-only capacities, stage differences and memory/latency
tradeoffs are documented in `2026-10-19-usb-preferences.md`. Android saves apply
to the next worker; host flags apply at broker startup and each original link
retains that policy. Preparing settings does not restart active media. No live
host/app restart or hot reload is claimed. Android's native timeout covers more
setup stages than the host TLS-specific knob; discovery, READY, I/O and retirement
remain explicit separate budgets. Inventory/fallback implementations also differ
and are documented without claiming identical native timing.

Host T1103 tests retain unrelated admitted camera protocol traffic when another
resource expires. T1100/T1102 retain original native admission/identity guards
through actual creator/thread/task retirement. T1105 tests delayed approval and
selected original TLS expiry independently. Android T1099 tests keep active
workers while saved future settings change and quarantine unconfirmed writes.
Their implementation artifacts contain individual native production-function
counters; this test/documentation item adds no production function requiring a
new counter. Previous full Android implementation acceptance remains 2345 tests.

The fixed test resource has ten lines/nine records and bounded scalar fields.
Rust includes it at compile time; Android closes each resource reader with `use`.
Parsed immutable specs retain no mutable input map or reader. Test storage/native
API calls reuse existing exclusive admission and actual-return lifetime rules.
The fault runner reuses T1079's original process/group/log cancellation/join
sponsor and restores its exact fixture in `finally`; no elapsed kill cap is used.
All temporary streams and original process results remain contained until settled.

The T898 draft was preserved in a named local stash while its newly discovered
TLS prerequisites were implemented. Restoration retained both the new T1105
mapping regression and the shared test module; no existing test was discarded.
Physical media, real app-restart persistence, native provider qualification and
unsupported backends remain T846/T863 and dependencies. Hosted acceptance is not
unrestricted native safety or physical USB support. No battery/performance gains,
new hardware campaign, installation, repository push or network transfer is claimed.
