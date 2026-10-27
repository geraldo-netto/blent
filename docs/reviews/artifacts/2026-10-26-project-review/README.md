# Project review — 2026-10-26

The October 26 review recorded seven findings with reproductions and explicit suite failures. Later individual fixes carry their own evidence; this review was not exhaustive platform or lifetime certification.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Reviewed commit `0cc6148a`. Seven findings are recorded in `TODO.md`; production
code and permanent tests are unchanged. Existing owner-directed EVDI ledger
removals and unrelated untracked evidence were preserved. No downloads, uploads,
pushes, real ADB connections or physical tablet operations were performed.

This was a broad source and automated-test review, not an exhaustive proof that
every source line or platform is correct. Inspection covered shared retirement,
configuration and wire contracts; host audio/video/camera and ADB ownership;
Android transport, decoder format, activity delivery and update parsing; GUI
lifecycle/settings adapters; native USB/capture helpers; packaging, CI and current
architecture documentation. Windows adapters received static inspection only.
Hosted Android tests do not establish physical Android acceptance. Existing
Windows/device acceptance limitations and declined platform/performance work
remain unchanged. Whole-project per-function coverage and sanitizers were not
remeasured; passing suites are not a substitute for those measurements.

| Finding | Severity | Evidence and impact |
|---|---|---|
| T1260 | high | `Cleanup::start` discards its native JoinHandle and publishes completion before TLS destructors finish. The real-crate probe blocks a TLS destructor and observes both completion and reuse of a capacity-one pool. Native cleanup threads can outlive their resource admission. |
| T1261 | medium | The release ABI gate checks five files while the bundle ships seven. Injecting GLIBC_2.40 requirements into either USB binary is accepted without inspecting that binary. This bypasses the advertised 2.36 floor. |
| T1263 | medium | The full Android suite fails the original ordered activity regression while waiting for a display request. Its standalone cases and a focused rerun of the complete prelude pass. This is a confirmed intermittent test failure; a production cause is not established. |
| T1264 | medium | Both full native audio CLI tests use obsolete same-port ADB fixtures. Their simulated tablet connects to the reserved device-side port rather than the host-side destination. Correcting only fixture route serialization/translation in temporary copies passes both native scripts. |
| T1265 | low | The production Android HTTP release adapter accepts `{"tag_name":"v9.0.0"} garbage` and offers 9.0.0 on hosted API 27/34. The host and shared response corpus reject the identical malformed document. |
| T1262 | low | Initial Android greetings without encoded dimensions still receive panel/default dimensions explicitly for old hosts. Current host responses always provide these fields, conflicting with the no-compatibility policy. Partial updates to an established format remain a separate legitimate behavior. |
| T1259 | low | `platform-linux` is explicitly a compatibility-only Cargo feature alias, with no current source caller. Its manifest and architecture description conflict with the no-compatibility policy. |

The ledger contains the exact locations and completion conditions, including
permanent regressions required with behavioral fixes. Review probes below are
supporting evidence; they are not substitutes for future normal-suite regressions.
The two compatibility findings are independent removals. T1260 is one cohesive
worker/admission lifetime outcome: splitting the exit fence from capacity release
would leave the bug unresolved.

## Automated results

| Execution | Result |
|---|---|
| Common library, offline locked Cargo | 401 passed |
| GUI, offline locked Cargo | 116 unit tests and 1 private native startup test passed |
| Host all-features library and binaries | 1,422 passed; 5 existing opt-in benchmarks ignored |
| Selected host integration/tooling targets | 48 passed; 4 failed |
| Android complete debug JVM suite | 2,701 passed; 1 failed out of 2,702 |
| Focused Android rerun | All 3 existing ordered tests passed; 2 temporary release probes confirmed malformed acceptance |
| Temporary corrected native audio fixtures | Both scripts passed, including unclocked/clocked PCM and retirement checks |

Host failures comprise the two T1264 audio cases and the pre-existing T102/T302
distribution fixture failures. The latter omit `capture_activity.c`,
`capture_control.c` and `idle_control.c` from their private copied tree. Their
failure output is retained, but the owner-dropped EVDI findings were not reopened.
The aggregate host command exited 101 and the full Android command exited 1;
neither full run is represented as green. The extra nested one-case host child
result is already represented by its parent regression and is not counted twice.

`native-host.log.gz` records the SDK/codec/cache paths and full Cargo command.
It ran `scripts/ci/test-host-offline.py` with the installed Freedesktop 25.08 SDK,
matching cached codecs, libclang 18 and real private PipeWire/X11 fixture tools.
Targets were `--lib --bins --test activity_dpms --test audio_native --test
linux_lifecycle --test wifi_setup --test shared_encode_replay --test tooling`,
with `--all-features --no-fail-fast -j 2`. Common and GUI commands were
`cargo test --offline --locked -p blent-config --lib` and
`cargo test --offline --locked -p blent-gui -j 2`.

Android used `./android/gradlew -p android --offline --no-daemon` and the existing
external offline init script at
`/backups/disk2/blent-work/2026-10-21-offline-reconnect/T1239/offline-tests.gradle`.
That script selects cached Robolectric SDKs, a 2 GiB heap and per-test progress
reporting. The full task was `:app:testDebugUnitTest`. The diagnostic init script
in `/backups/disk2/blent-work/2026-10-26-review/review-init.gradle` additionally
includes an external temporary test source directory; the focused run selected
`com.blent.SavedUiOrderTest` and `com.blent.ReleaseReviewProbe`.
`android-order-first.xml.gz` preserves the original failure before the focused
run replaced Gradle's test report. `android-order-rerun.xml.gz` preserves the
passing retry without concealing the original failure.

## Reproductions and ownership

`cleanup_tls.rs` links the current compiled `blent_config` crate and uses one
pool slot, one cleanup closure and one gated TLS destructor. The destructor
signals entry before the observer checks completion/capacity. The probe then
releases the gate and waits for its final destructor signal. Its output is in
`cleanup_tls.log`. It establishes premature admission reuse, not an observed
use-after-free or an unrestricted leak proof. The production closure's end is
not a native exit fence; the exact cleanup worker must remain owned through join.
Cleanup liveness remains conditional on the native destructor eventually exiting.

The original command used installed `rustc --edition=2021`,
`--extern blent_config=target/debug/deps/libblent_config-833221ed7e473a0c.rlib`
and `-L dependency=target/debug/deps`. Build artifacts may have different hashes
after rebuilding; select the corresponding current crate artifact. Probe output
and executables were owned by `/backups/disk2/blent-work/2026-10-26-review`.

Run `python3 docs/reviews/artifacts/2026-10-26-project-review/portability_omissions.py`
to reproduce T1261. It invokes the actual verifier with bounded synthetic
`objdump`/`readelf` results and all seven temporary bundle files, not real binaries
requiring a newer libc. Each temporary directory and mock is scoped and released.

`ReleaseReviewProbe.kt` invokes the actual HTTP adapter through an in-memory
OkHttp interceptor, with no network transfer. The response body is owned by the
production callback. The probe cancels its call, shuts down and awaits its
dispatcher, then evicts pooled connections. Its two SDK outputs are retained in
`release-probe.xml.gz`. The normal common library test passes the shared strict
response corpus containing the same malformed input.

`audio_fixture_translation.py` copies only the four existing audio fixture
modules into an owned temporary directory. Two textual substitutions preserve
the requested reverse route and translate the invitation port to its destination.
It runs the existing native helper/host binaries with isolated PipeWire servers
and no real ADB tool. The original fixture assertions and process cleanup remain
in force. `audio-fixture-translated.log` records both exits as zero. The runner
uses the local SDK/cache paths recorded above and leaves repository files intact.

`probe-complexity.json` records all six parsed functions/lambdas in the authored
probes, maximum score 5, using the project's local cyclomatic checker. The audio
runner has no function definitions. `results.json` records parser versions,
source hashes and suite outcomes. No parser failures or absent native evidence
are treated as passing results. The existing whole-project complexity evidence
at this reviewed commit is in `../2026-10-25-t1090/complexity.log.gz`; it was not
rerun merely to duplicate the unchanged-source measurement.

</details>
