# Review reproductions — 2026-11-13

Baseline and finding-source hashes: [source-manifest.json](source-manifest.json).
Review: [2026-11-13-project-review.md](../../2026-11-13-project-review.md).

The Rust probes include current production modules by absolute path from
`/home/netto/Desktop/projects/blent`. They inject synthetic command/identity
adapters and use temporary configuration directories. They do not run ADB.
The USB-only `valid_serial` probe stub is unreachable: these probes use the
production transport constructor instead. Compilation warnings describe unused
production functions in these deliberately narrow harnesses.

To reproduce in an external workspace at the same checkout path:

```sh
probe=/home/netto/.cache/blent-review-20261113/replay
artifacts=docs/reviews/artifacts/2026-11-13-project-review
mkdir -p "$probe/src/bin"
cp "$artifacts/probe-Cargo.toml" "$probe/Cargo.toml"
cp "$artifacts/probe-Cargo.lock" "$probe/Cargo.lock"
cp "$artifacts/identity-probe.rs" "$probe/src/main.rs"
cp "$artifacts/discovery-probe.rs" "$probe/src/bin/discovery.rs"
cp "$artifacts/starvation-probe.rs" "$probe/src/bin/starvation.rs"
cargo run --offline --locked --manifest-path "$probe/Cargo.toml" --bin blent-review-probe
cargo run --offline --locked --manifest-path "$probe/Cargo.toml" --bin discovery
cargo run --offline --locked --manifest-path "$probe/Cargo.toml" --bin starvation
```

The dependency cache must already contain the locked versions. The identity
probe prints stale-cache retirement authorization. The discovery probe prints
one query across two observations despite changed adapter identity. The
starvation probe prints two deadline expirations and zero healthy-device queries.
These demonstrate the existing defects and exit successfully after reporting
them; they are not passing correctness assertions or permanent regression tests.

`verification.json` records the separately executed normal suites and checker.
Compressed logs preserve their full results; the complexity log retains every
measured function and score. Counts from child fixture executions are excluded
from the main Rust suite totals.
