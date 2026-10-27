# T1090: retained temporary ADB media mappings

One retained owner manages Blent's temporary ADB mappings under the explicit exclusive-endpoint policy. Private CLI/server regressions passed; selected-tablet acceptance remains T1258 and whole-process crash recovery is outside this claim.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Owner decision 2026-10-25 permits exclusive management of Blent-created temporary
endpoints through actual cleanup. Unrelated ADB activity remains allowed. This
resolves the ownership decision missing from the historical
[2026-10-19 audit](../2026-10-19-t1090-audit/README.md); that audit remains historical.
Verification completed 2026-10-26, offline, without real ADB/tablet interaction,
downloads, uploads or repository transfers.

The current independent contract and ownership assumptions are in
[adb-media-mappings.md](../../../adb-media-mappings.md). Camera, audio and the
camera feedback probe use one shared Linux owner. Creation admits bounded capacity
and publishes the original retirement proof before native effects. Both actual
listener descriptors remain funded through cleanup and worker join. Mutations
are serialized, native children/collectors are fenced, uncertain effects stay
owned, and unrelated or foreign routes are preserved. No elapsed deadline grants
permission to reuse an endpoint.

## Permanent regression evidence

The normal Cargo suites retain the T1090 tests in
`host/tests/adb_mapping_retirement.rs`, `host/src/adb_mapping/tests.rs`,
`host/src/audio/connection_tests.rs`, `host/tests/camera_probe.rs` and
`common/src/adb_reverse/temporary/tests.rs`. They execute production Rust against
a private native Python CLI and a separately owned/reaped fake server. They do
not invoke the installed ADB server or access a device. Fixture control files
force observable boundaries; temporary directories are removed after completion.
The constructor fault test changes only its private subprocess file limit and
restores it before assertions.

Compressed red logs preserve the initial four production failures (drop, partial
creation, cancelled creation and failed-close ownership), plus failures found
while developing the same lifecycle: uncertain removal reissued, a never-spawned
removal incorrectly quarantined, audio returning before partial cleanup, local
destination reuse and runtime-constructor resource/admission destruction order.
`red-endpoint-fence.log.gz` is a deliberate faulty-control run with endpoint
release removed; its regression detects the retained descriptor. The faulty
change was restored. Red source locations predate the final module refactor;
case names identify the retained tests. `red-audio-owner.log.gz` and
`red-runtime-admission.log.gz` include both parent and child diagnostics for one
failed case, not two independent cases. `red-camera-helper.log.gz` records the
broader run catching a test-only diagnostic wrapper error; the retained T609
regression passes after correcting that helper without weakening its oracle.

The policy exhausts all 12 creation-state/route combinations. Bounded malformed
listing/port tests exercise invalid and boundary values. Native cases also cover
late server effects after CLI failure, caller cancellation/runtime destruction,
exact invitation acknowledgements, queue cutoff, capacity exhaustion, malformed
observations, repeated close and listener/worker descriptor release. The model is
finite; it does not prove arbitrary native behavior. Conditional eventual cleanup
requires eventual observable effects, valid listings and native joins. Worker
panic or permanently unknown effects retain funded capacity rather than claim
settlement. Whole-process crash recovery is outside this process-lifetime claim.

## Reproduction and measurements

Use installed Rust/LLVM tools and the locked offline dependencies:

```sh
cargo llvm-cov clean --workspace --offline --locked
cargo llvm-cov test --offline --locked -p blent -p blent-config --lib --features blent/accessory-transport --no-report
cargo llvm-cov test --offline --locked -p blent --test adb_mapping_retirement --test camera_probe_stop --test camera_probe --features accessory-transport --no-report
cargo llvm-cov test --offline --locked -p blent --lib --no-report
cargo llvm-cov report --offline --locked --lcov --output-path coverage.lcov
cargo clippy --offline --locked -p blent --lib --features accessory-transport --no-deps -- -D warnings
cargo clippy --offline --locked -p blent-config --no-default-features --no-deps -- -D warnings
python3 scripts/complexity/check.py --verbose
```

The recorded run used one external `CARGO_LLVM_COV_TARGET_DIR` for the three
coverage cohorts. `changed-functions.json` identifies production functions whose
source differs from the parent commit; test-only bodies and the pre-existing
benchmark/development-script exemptions follow the project inventory. Source
hashes are checked against the frozen measurement scope. `function-coverage.json`
contains individual executable-line counters and the 80% gate; compressed LCOV
preserves the native counters. This is changed-function evidence, not a claim
that every unchanged function meets coverage. Native Clippy checks are scoped
with `--no-deps`; the unrestricted command is blocked by an unchanged EVDI
`needless_question_mark` at `common/src/linux/capture_control.rs:143`, retained in
`clippy.log.gz`. No lint suppression or EVDI change was added.

`complexity.json` contains per-function local cyclomatic measurements for touched
sources, tests and the fixture. `complexity.log.gz` retains the whole-repository
check. Tool/parser versions, exact counts and source hashes are in `results.json`.
The local environment uses cached parsers recorded by the prior T1221 evidence;
no tools were downloaded. Physical ADB/Android acceptance remains explicit under
T1258, requiring a selected tablet and authorization for that separate trial.
Interface readiness and private fixtures do not establish that native evidence.

</details>
