# Follow-up review evidence

See [the report](../../2026-11-13-project-review-followup.md) and
`verification.json` for the selected commands, counts and limitations.
`source-manifest.json` pins the reviewed revision and finding-source hashes.
Logs are retained with deterministic gzip encoding where large.

The probes ran in a temporary crate outside the repository. Its manifest and
lock are `probe-Cargo.toml` and `probe-Cargo.lock`. To reproduce, copy these as
Cargo.toml/Cargo.lock into an owned temporary directory, create `src/bin`, and
copy each `*-probe.rs` there as `identity.rs`, `inventory.rs` and `reconnect.rs`.
Copy both `.inc` files alongside them. Run `cargo run --offline --locked --bin
NAME` from that crate, using an owned target directory. The manifest and source
attributes refer to the recorded checkout path; adjust paths for another checkout
only after checking its source against the manifest.

`linux_identity_body.inc` is byte-for-byte the reviewed production function.
`network_body.inc` is the reviewed production module with only `mod queries`'s
source location made explicit for external compilation. Those relationships were
checked again after the probes completed. The inventory probe calls the compiled
production library directly. No production code or permanent tests were changed.

The final reconnect log includes both recovery on later polls and original
simulated-route cleanup. Its unused-method compiler warnings come from including
the production route/network modules in a narrow probe; they are not test
failures or newly reported production defects.

Android failure XML preserves the exact expected/actual remembered-approval
mismatch and the ordered-wrapper failures. `android-results.json` lists all
366 reported suites. Existing regressions remain unchanged for the future fix.
