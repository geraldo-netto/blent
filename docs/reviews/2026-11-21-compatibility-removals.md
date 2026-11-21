# Compatibility-only removals — 2026-11-21

Scope: current Blent production, tools and packaging. The no-backward-compatibility rule is saved in local AGENTS.md, persistent maintainer preferences and CONTRIBUTING.md. Hardware capability selection, current recovery and historical reports are distinct from obsolete caller support. This audit is in progress; installed sessions are unchanged during source verification.

## T1534 — position aliases

Removed undocumented `top`/`bottom` aliases. Current values remain right/left/above/below; the parser rejects obsolete aliases, and persisted invalid values follow the existing invalid-value policy. The permanent rejection regression failed before removal. All 64 selected model/storage/runtime cases pass; the maintained parser has 100% executable-line coverage. The parser and changed regressions have complexity <=9 (Rust syn 2.0.119 project checker; Rust 1.100.0 and cargo-llvm-cov 0.8.7). The parser borrows call-local text and returns an enum; no native resource or retained state is added.
