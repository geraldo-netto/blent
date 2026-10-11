# Per-function coverage (T497)

The accepted gate combines native Linux and Windows Rust, Android, capture C
and essential-script counters. Require **80% executable lines for every maintained
function/method**, plus permanent bounded invalid-value and out-of-bounds tests.
Missing counters, changed source bytes and empty platform scopes fail. See the
[current evidence](../../docs/reviews/2026-09-26-function-coverage.md); the
[2026-09-19 report](../../docs/reviews/2026-09-19-function-coverage.md) is historical.

The requirement covers Rust applications, the Android app, the C EVDI helper,
and essential installation, EVDI setup and packaging scripts. Benchmarks in any
language (including their Rust example adapters) and nonessential development
tools are exempt. The script inventory is
explicit in `inventory.py:essential_script`: packaging sources plus the release
builder, bundle staging, distribution-document copying, EDID generation, APK
verification, installer, EVDI setup, desktop/systemd launch writers and the CI
artifact/Arch builders and package ABI validation. All tests already developed,
including benchmark and development-tool tests outside this scope, remain in
the normal automated suites.
The capture C gate currently covers every maintained function in `host/evdi/`;
this is a scoped result, not a whole-project percentage.

Install the reporting tools in an isolated environment:

```sh
python3 -m venv /tmp/blent-coverage-venv
/tmp/blent-coverage-venv/bin/python -m pip install -r scripts/coverage/requirements.txt
/tmp/blent-coverage-venv/bin/python -m unittest discover -s scripts/coverage -p 'test_*.py'
```

## Capture C gate

```sh
/tmp/blent-coverage-venv/bin/python scripts/coverage/capture_check.py /tmp/blent-capture-coverage
```

Use a new output directory for each run. The command runs the normal helper,
conversion, EVDI-module and frame-retirement test targets with GCC counters,
preserves coverage notes before the fixtures remove their temporary executables,
and rejects changed sources. It writes the test log, raw evidence, source manifest
and `report.json`. Every capture function must have at least 80% executable-line
coverage. Missing counters fail the gate. These tests simulate EVDI; they do not
attach a virtual display or validate the unresolved Xorg crash in T222.

## Optional GPU C gate (T766)

```sh
/tmp/blent-coverage-venv/bin/python scripts/coverage/gpu_check.py /tmp/blent-gpu-coverage
```

This separate gate includes every `host/gpu/` production function, including
the opt-in executable. It requires installed FFmpeg/VAAPI/X11 development
packages, GCC/gcov and Xvfb; it never installs dependencies. `PKG_CONFIG_PATH`,
`GPU_CFLAGS` and `GPU_LIBS` can select an existing SDK. Sources are fingerprinted
before execution and must remain unchanged until collection completes.

Policy tests, isolated X11 tests and mocked DRI3/VAAPI/codec lifecycle tests run
under ASan/UBSan with native GCC counters. Every function must reach 80%; missing
counters fail. The mock tests validate Blent's boundary calls and ownership;
they do not establish physical GPU driver compatibility or remove the prototype
label. CI includes these counters in the combined gate with a second `--gcov`
input. No real display or render device is used.

## Owned C boundary testing (T767)

The normal Cargo integration tests retain deterministic invalid-input cases and
bounded mutations. The following map identifies external inputs and the internal
ownership contracts exercised by the fixtures:

| Boundary | Permanent coverage |
|---|---|
| Helper argv, EDID files, device selection, capacity/idle control files and FIFO descriptors | `evdi_helper.rs`, `helper_arguments.c`, `coverage_capture.c`, `fifo_demand.c`: malformed integers, missing fields, file/type/ownership checks, failed initialization and teardown. |
| EVDI mode/update/cursor callbacks | `c_boundary_fuzz.c`, `cursor_overlay.c`, `failed_mode.c`, `damage_regions.c`: signed extremes, geometry/stride/length, clipped cursor pixels, allocation failures and queued events after failed mode retirement. |
| Raw control packets, descriptor negotiation and shared frame slots | `c_boundary_fuzz.c`, `raw_producer.c`, `shared_capture.c`, `shared_damage.c`: packet mutations, truncation/ancillary descriptors, bounded layouts, generation changes and leased-slot immutability. |
| Conversion, damage spans and writer/pool ownership | `conversion_test.c`, `frame_retirement_test.c`, `evdi_modules_test.c`: independent pixel expectations, padded stride, worker failure, bounded rectangles, retirement and shutdown. Internal pixel pointers must address initialized allocations of the declared size; callers validate public geometry first. |
| Optional GPU argv, X11 events, DRI3 metadata and codec lifecycle | `gpu_options.c`, `gpu_x11.c`, `gpu_lifecycle.c`: invalid options, cursor intersection properties, fragmented damage, metadata bounds and failed native transitions with explicit ownership counters. |
| Benchmark argv and JNI direct buffers | `benchmark_arguments.c`, `c_benchmarks.rs`, `rect_native.c`: complete numeric parsing, overflow/odd geometry, codec/offset/length/capacity limits, independent hash oracle and unavailable EGL extensions. JNI contexts are NULL or live handles returned by create; forged handles are outside that caller ownership contract. |
| Retained guest KMS probe | `kms_probe.c`: allocation, plane selection and transition failures using fake DRM interfaces; no real display is opened. |

T767 adds 8,192 deterministic EVDI mutations, 1,521 structured dimension/position
combinations, single-bit control-packet mutations, and 8,192 JNI argument samples
plus explicit range endpoints. `c_fuzz_driver.c` exposes the same EVDI input
boundary to optional local clang libFuzzer campaigns. On 2026-10-09, 100,000
additional seeded runs passed ASan/UBSan. These are bounded samples, not an
exhaustive proof for every possible parameter or thread interleaving.

All fixtures preserve valid backing allocations and initialized synchronization
objects. They do not manufacture dangling pointers or call internal functions
with invented owner state. Dedicated regressions exercise supported lifetime
transitions and injected failures instead. JNI tests use installed JDK/EGL
headers and fake codec/EGL interfaces: they test owned wrapper logic, not codec
implementations or Android device acceptance. GPU tests use private Xvfb and
fake native adapters, not physical driver acceptance. External libraries and
kernel code remain outside this audit; no kernel sanitizer result is claimed.

## Python and shell collection

```sh
/tmp/blent-coverage-venv/bin/python scripts/coverage/script_check.py /tmp/blent-script-coverage
```

This runs the normal isolated tooling regressions and Python test suites,
including retained tests outside the coverage scope. Only the essential-script
inventory is subject to the threshold.
Packaging prerequisites, including `rpmbuild`, must be available. `--report-only`
allows a report containing gaps to be written without failing the command; the
report still says `passes: false`. Remove that option to enforce the threshold.
Test failures always fail collection.

Python uses coverage.py subprocess counters and separate function-call evidence.
Loading a definition, including a one-line function, is not counted as invoking
it. Byte-identical copied fixtures are attributed to their original sources by
SHA-256; changed or ambiguous copies do not receive credit. Bash uses native DEBUG
locations matched to the source hash. Extracted Bash functions also count when
their entire definition is byte-identical to exactly one maintained function;
the collector records a body hash and original line bounds before the temporary
fixture disappears. Changed or ambiguous bodies receive no credit, and failed
attestation fails the report. The trace records only source hashes,
filenames and line numbers, never expanded commands, arguments or credentials.
Shell coverage applies to Bash executions; code executed only by another shell
remains unmeasured. Embedded Python and RPM scriptlet functions remain in the
inventory even when the collector cannot yet measure them.

## Linux Rust gate

Install `cargo-llvm-cov` 0.8.7 and the Rust `llvm-tools-preview` component.
With the snapshot described below and a new evidence directory, run both normal
workspace configurations against unchanged production sources:

```sh
CARGO_LLVM_COV_TARGET_DIR="$(mktemp -d /tmp/blent-llvm-default.XXXXXX)/target" \
  cargo llvm-cov --locked --workspace --lcov --output-path /tmp/blent-default.lcov
CARGO_LLVM_COV_TARGET_DIR="$(mktemp -d /tmp/blent-llvm-features.XXXXXX)/target" \
  cargo llvm-cov --locked --workspace --all-features --lcov --output-path /tmp/blent-all-features.lcov
/tmp/blent-coverage-venv/bin/python scripts/coverage/rust_check.py \
  --manifest /tmp/blent-coverage-manifest.json \
  --lcov /tmp/blent-default.lcov --lcov /tmp/blent-all-features.lcov \
  --output /tmp/blent-rust-coverage
```

T638: collect each source revision in a fresh LLVM target directory. Reusing
old instrumented executables can merge obsolete function/line mappings even
when current source fingerprints match. Do not repair such counters by dropping
uncovered lines; discard the contaminated report and recollect. The Windows CI
jobs use fresh runners and separate coverage outputs. Preserve fingerprints from
before compilation and reject later source changes as usual.

The scoped Linux gate writes `linux.json`. `all-platforms.json` retains every
foreign-platform function and fails until native coverage is supplied to the
combined reporter. Classification evaluates explicit `cfg` target predicates,
including nested `all`/`any`/`not`, regardless of argument order. Only code
provably unavailable on Linux leaves the Linux gate; unknown feature predicates
and shared module references stay in scope. Guarded module descendants inherit
the restriction. Missing Linux functions always fail, and foreign-platform
counters are rejected from a Linux collection. This gate does not establish
Windows or macOS runtime support or coverage (T493/T497/T583). CI uploads the
manifest, raw counters, reports and logs as coverage artifacts.

## Combined native reports

Before measuring, create a source snapshot:

```sh
/tmp/blent-coverage-venv/bin/python scripts/coverage/report.py snapshot /tmp/blent-coverage-manifest.json
```

Keep sources unchanged until the measurements and report are complete. Collect
Rust with the fresh-target collector below, once with `--all-features` and once
without it; preserve both reports to cover the CLI-only build.
GCC records use `compiler.py`, as demonstrated by the capture gate. Android uses
the regular Robolectric suite with a separate JaCoCo init script:

```sh
./android/gradlew -p android -I "$PWD/scripts/coverage/android.init.gradle" profileCoverage
```

The Android XML is written to
`android/app/build/reports/jacoco/profileCoverage/profileCoverage.xml`.
Enforce the method gate with:

```sh
/tmp/blent-coverage-venv/bin/python scripts/coverage/report.py check \
  --manifest /tmp/blent-coverage-manifest.json \
  --jacoco android/app/build/reports/jacoco/profileCoverage/profileCoverage.xml \
  --scope android/app/src/main/ --output /tmp/blent-android-coverage.json
```

This does not install an APK or use a physical tablet. Its scope is the main
Android app; separate benchmark applications are exempt.

Pass the native artifacts to `report.py check`; see `--help` for repeated LCOV,
GCC, Python and shell inputs. An explicit `--scope` is recorded in the output.
Without one, every maintained production function in the required scope is checked. The source
inventory excludes generated files, dependencies, caches/build outputs and test
fixtures, including Rust test-only modules. An absent measurement fails; there
is no aggregate-percentage substitute or silent platform exclusion.

Kotlin uses authored PSI functions, accessors, constructors and callbacks compiled
as separate methods. T848 inventories primary construction, instance property/init
bodies, anonymous-object initialization, named-object/companion and top-level static
initialization, and enum entry arguments. One source initialization owner includes
all of its property/init bodies; every mapped native method variant must pass.
Secondary-only classes retain shared initialization within each secondary
constructor's native counters. Initialization without matching native method
counters fails; coverage of unrelated methods cannot replace those counters.
T966 omits empty init blocks and property literal-null defaults (including
parentheses): these emit no authored JVM initializer instructions. Actual compiler
fixtures retain singleton INSTANCE setup but prove no property assignment; native
JaCoCo filters that generated setup. Calls, delegates, parameter defaults and
nonempty init blocks remain inventoried. Missing executable counters still fail.
Native JaCoCo filters remain authoritative for compiler-generated scaffolding:
retained default-constructor variants with authored counters must also pass. Per the maintainer's scope, compiler-inlined lambda bodies
count within their enclosing function/method. Lambda body-entry lines disambiguate
callbacks declared on a parent's first statement line. Native method counters
prevent allocating a callback from counting as executing it. Multiple compiled
variants must all pass. JaCoCo 0.8.15 applies its supported Kotlin inline and
Compose filters, excluding generated scaffolding; see its
[change history](https://www.jacoco.org/jacoco/trunk/doc/changes.html).

The Linux daemon lifecycle regression requires a private PID namespace. It uses
`unshare` by default. The portability CI container supplies its own namespace and
sets `BLENT_TEST_PRIVATE_PID_NAMESPACE=1`; do not set that variable in a normal
desktop session. The regression starts only its own isolated daemon, uses fake
ADB/helper commands and signals its owned child directly.

Runtime-directory failure tests also require a private mount namespace and mask
`/run/user` with a temporary filesystem. Invalid XDG paths can fall back to the
real user runtime directory; setting a private HOME alone does not isolate that
case. These tests may use the existing isolated CI container when its explicit
private-namespace marker is present. A namespace setup failure fails the test.

Native Windows ACL/instance-ownership acceptance passed under T493; see the
[retained native counters](../../docs/reviews/2026-09-26-windows-foundation.md).
Wine loses the protected-DACL flag and remains unsuitable for these positive
cases. Keep unmeasured functions visible; do not treat Linux counters or Windows
cross-compilation as native Windows coverage. The Windows workflow preserves
LF source bytes and uploads LCOV plus SHA-256 source fingerprints. Windows
relative backslashes are accepted by the importer; absolute drive paths require
an explicit source prefix. Source bytes must match exactly before combining.


## Combined native gate

`build.yml` joins Linux, Android and the reusable native Windows workflow before
running the combined gate. Windows collection includes common, host **and GUI**
sources, tests and SHA-256 fingerprints. Both Linux configurations use separate,
fresh LLVM target directories, including when Cargo build caches are restored.

To combine downloaded artifacts in the same layout as CI:

```sh
/tmp/blent-coverage-venv/bin/python scripts/coverage/report.py check \
  --manifest evidence/host/blent-rust-coverage/manifest.json \
  --lcov evidence/host/blent-rust-coverage/default.lcov \
  --lcov evidence/host/blent-rust-coverage/all-features.lcov \
  --lcov evidence/windows/windows-common.lcov \
  --native-sources evidence/windows/windows-sources.json \
  --jacoco evidence/android/jacoco.xml \
  --gcov evidence/host/blent-capture-coverage/native \
  --python-json evidence/host/blent-script-coverage/python.json \
  --python-calls evidence/host/blent-script-coverage/calls \
  --shell evidence/host/blent-script-coverage/shell \
  --rust-platform linux --rust-platform windows \
  --output evidence/combined.json
```

Windows scope requires nonempty native fingerprints covering every maintained Rust
counter file; test fixtures retain their existing inventory exclusion. Missing, partial or mismatched attestation fails. Native collection
must execute on Windows; GNU linking and Wine never replace it.

`--rust-platform` is explicit and recorded in the report. Only functions provably
unavailable on every selected platform leave that gate; unknown predicates remain
in scope. Excluded functions remain in `excluded_platform_functions` with their
missing counters. The single Darwin scheduling function belongs to declined T584;
Linux/Windows results do not imply macOS support. Omitting this option retains all
platforms and fails when Darwin counters are missing.

Archived recipe tests use synthetic `<archived recipe: ...>` code filenames.
They still execute their unchanged security assertions, while coverage no longer
mistakes archived ASTs for nonexistent production files. A permanent native Python
reporting regression verifies this boundary; no production counters are ignored.

## Fresh local Rust collection (T880)

Use a new output directory for each invocation:

```sh
python3 scripts/coverage/rust_collect.py --output /tmp/blent-rust-default
python3 scripts/coverage/rust_collect.py --output /tmp/blent-rust-all --all-features
python3 scripts/coverage/rust_collect.py --output /tmp/blent-rust-common --package blent-config --lib
```

The collector enforces offline, locked Cargo resolution and creates its own fresh
target/build tree, overriding inherited reused-target paths and profile output.
It never cleans or reuses a caller's existing directory. Package and test options
accept names; use `--filter` for a Rust test-name filter. Arbitrary Cargo argument,
output-path and target-dir passthrough is intentionally absent. CI already uses
fresh target directories.

The output retains `manifest.json`, `rust.lcov`, Cargo logs, raw profiles/native
objects and SHA-256 attestation of sources, report, collector and owned objects.
Source edits, inventory changes, escaped native-object links, failed tests and
empty reports prevent success attestation. Native counters are unchanged; run
`report.py check` separately to enforce per-function coverage. A successful
collection does not itself claim the coverage threshold or platform support.
Keep the directory for review, then delete only your own evidence when no longer
needed; fresh compilation trades extra local disk/build time for unambiguous
mapping ownership. Missing cached dependencies fail offline; this tool does not
authorize downloads.

T965 shares authored PSI ownership with the complexity inventory through
`scripts/kotlin/AuthoredKotlin.kt`. Native method attribution, generated-code
filtering and per-function thresholds remain unchanged; the compiled inventory
cache fingerprints both the reporter and ownership source.

T970: `--feature NAME` selects an explicit Cargo feature and can repeat. For
example, `--package blent --lib --feature accessory-transport` collects the host
USB adapter without enabling `native-audio`; unavailable audio development SDKs
remain visible under T835. Feature names have the same bounded typed validation
as package/test names. Offline resolution, fresh target ownership and source/
object attestation remain mandatory.
