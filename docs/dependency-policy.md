# Independent contracts and dependency versions

Owner decision 2026-11-22: define Blent's public/domain API contracts independently
of their implementations and third-party libraries. Keep requests, responses,
errors, schemas and ownership/lifetime requirements in focused contract modules.
Implementations depend on those contracts through explicit adapters. Keep vendor
handles, concrete library types and runtime assumptions inside the adapter that
owns them; do not expose them through shared domain contracts. Test the contract
independently and verify each adapter against it. A library upgrade should update
its adapter and current callers without changing unrelated domain code.

Prefer one resolved version of each library within a shipped target/feature set.
Whenever adding or upgrading a dependency:

- Inspect direct and transitive normal/build dependency graphs for each shipped
  build. Count distinct versions per package name; distinguish build-only inputs,
  other-target lockfile entries and libraries actually selected for the bundle.
- Reuse shared dependency declarations where appropriate; remove unused direct
  dependencies and keep test-only dependencies in their test scope. Upgrade
  current consumers/adapters together where this eliminates older versions.
- Record unavoidable duplicate versions in TODO.md with the exact consumer chains,
  incompatible constraints, affected build and concrete removal prerequisite.
  An adapter does not override a third-party consumer's dependency requirements.
  Do not hide duplication with aliases, compatibility shims, forced incompatible
  substitutions, notice deletion or feature removal. Preserve actual platform
  capabilities, native ownership, tests and explicit work pauses.
- Verify affected contracts, adapter lifecycle/regression tests and release builds;
  regenerate notices and source inventories from the dependencies actually used
  by the rebuilt bundle. Do not claim fewer runtime libraries from lockfile counts
  alone, or performance gains without measurement.

