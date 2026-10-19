# T1115 original FIFO policy integration

Permanent red evidence reproduces old 1000 ms rejection, invalid draft geometry,
unknown viewer observation admission, mismatched encoder configuration, an
unqualified backend entering its trial, and stale publication after liveness
loss or receipt expiry during native waits. The final 58-test idle suite passes;
new tests remain in the normal suite with their T1115 IDs. Existing cancellation,
stale generation, exhaustion and original native cleanup tests remain intact.

Actual C reader admission precedes Rust publication on an inherited original
FIFO fd. It accepts 500/1000 ms and clear, with ASan/UBSan/LSan enabled. Its command
fixture rejects invalid argc, malformed/overflow/closed descriptor values, all
255 unsupported command bytes and EOF. Production native C is unchanged from
T1119/T1120/T1121; their original-writer tests/counters remain the native side of
this integration evidence. Native source hashes also include the probe and
idle_control.c/h. No physical EVDI, tablet or deployed process was touched.

Offline full command:
`cargo test --offline --locked -p blent -p blent-config --bins --lib --features accessory-transport`.
Results: 904 host library, 590 host binary and 380 common library tests pass;
zero failures. Seven preexisting opt-in synthetic benchmarks remain ignored by
their normal suite definitions. Child-isolation test results in the log are not
counted twice. Ordinary offline `cargo check -p blent --bin blent` also passes
without warnings. This is the selected accessory feature set, not broad optional
SDK/all-features qualification (T835).

Native Rust coverage uses a fresh exclusively owned target, cargo-llvm-cov,
source/object hashes and raw LCOV. Independent source inventory covers all
production functions in idle.rs, directory.rs, staging.rs, policy.rs, subjects.rs,
common idle.rs and unchanged.rs: 93 functions, minimum 90.9091%, none below 80%.
An initial collection omitted common library targets and did not pass; its
failure and per-function report are retained. Corrected collector selects both
binary and library targets; it adds no exclusions or weakened oracles.

Two deliberately reverted checks fail their permanent regressions: trusting
receipt proof sampled before a native wait, and omitting admission revalidation
after staging IO. Finally restores exact bytes after each fault. All native
input hashes match the final coverage manifest before the restored full idle
suite passes. Fault subprocess ownership uses the existing process sponsor and
actual join, without an elapsed compilation cutoff.

Cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2 with repository branch counting:
206 scoped Rust functions, maximum 9. Clang 18.1.3 inventories all four C fixture
definitions; explicit C source counts are 8/3/7/5. Collector/audit/fault functions
also satisfy 9. These are source estimates, not native SonarQube evidence; T1019
remains unresolved. Rustfmt scoped check and clang-format 18.1.3 with LLVM base,
Allman braces, indent4/column100 pass. Repository has no .clang-format baseline.

Ownership, contracts, bounds, conditional cleanup and remaining adapter/UI/
physical limits are in the linked FIFO policy review. Temporary native target
objects stay local under /tmp, with their digests retained in the attestation;
no network transfer, installation, push or live host reload occurred.
