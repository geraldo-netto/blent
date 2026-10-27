# T827: enforce scoped contract boundaries

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

The normal host tooling suite runs retained T820/T821/T824 guards plus T827
forbidden-edge fixtures. Rules cover media/selection return edges, concrete capture
and persistence owners, source inclusion, and public C owner layout. Private C
headers have an explicit production-owner allowlist; adding another consumer
requires a reviewed rule change. Aliases, grouped imports and qualified type paths
have permanent negative fixtures; comments and permitted directions have positives.

`host/tests/contract_build.rs` compiles actual selection, media-storage, media,
capture and private-worker contract sources in a separate crate that declares no
native capture, selection workers, sessions, queue or filesystem adapter. All 11
contract tests pass. Portable common tests also pass with no default features;
existing CI retains that check and its WASM build. These guards supplement the
existing independent Android contract and lifecycle tests. No native or maintained
production code was excluded, nor were coverage/complexity gates relaxed.

The rules are deliberately scoped lexical checks, not a full language parser or
proof of SOLID. The independent build catches unresolved concrete dependencies;
behavioral contract tests remain necessary. [Validation logs](artifacts/2026-10-10-t827/).
