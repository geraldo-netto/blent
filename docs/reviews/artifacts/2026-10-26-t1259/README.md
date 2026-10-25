# T1259: remove obsolete platform-linux feature

Removed the compatibility-only Cargo feature and its current architecture prose.
No current caller selects it. Both offline locked checks passed:

- `cargo check -p blent-config --no-default-features --offline --locked`
- `cargo check -p blent-config --no-default-features --features platform --offline --locked`

No runtime functions, allocations or lifetimes changed. No new coverage or
complexity measurements apply. Existing permanent tests remain unchanged.
The accompanying project-review artifacts record the preceding review; their
findings are historical and resolution is tracked by individual commits.
