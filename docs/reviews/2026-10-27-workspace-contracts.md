# Workspace contracts (T1272)

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

Implemented independent device catalog, resource projection and display-control
request/snapshot contracts. Native adapters and workspace UI remain T1274/T1273.

Catalog identity is the validated transport identity, never the display name.
The caller owns each bounded catalog (1..1024 devices); projections borrow one
current observation for one frame. No owner observation means "not started here",
not proof that an external process is idle. Unknown/queued/retiring work cannot
be treated as free capacity. Active owners remain visible through capability loss.

Consuming a camera/audio controller returns its original retirement fence.
Production fences complete only after the existing funded worker reaper observes
native thread exit and joins it, including TLS cleanup. Dropping a view or changing
selection cannot retire that worker. Synchronous test backends return Complete.
This adds no native allocator, thread or alternate cleanup owner.

Display requests bind user intent to one owner nonce and exact tablet identity;
Stop also requires the original slot/generation lease. Snapshots contain at most
four distinct targets/slots. Acknowledgement alone is not retirement evidence.
These DTOs contain no OS, GUI, concrete slot or backend types.

Offline Linux evidence (Rust 1.98.1, cargo-llvm-cov 0.8.7, LLVM 22.1.8):

- Initial independent projection tests: 3 passed, 2 failed against the stub;
  implemented projection passes the same bounded state matrix.
- Strict inspect decoding regression failed for the unit variant; an empty
  struct variant rejects unknown fields and passes the retained regression.
- Nine contract tests plus two native controller-fence tests pass. The common
  suite previously passed 416 tests; GUI suite passes all 131 tests after updating
  the six synchronous fixtures for the required consuming operation.
- All 23 changed production functions have 100% executable-line coverage in the
  filtered common/host contract run. Source-hash manifests and LCOV are retained
  in the owned external work directory `2026-10-26-t1267-t1270`.
- Locked syn 2.0.119 complexity frontend measures 114 functions in the seven
  maintained source files, maximum 7; no suppression or metric exclusions added.

Fence tests establish adapter handoff and original admission preservation.
They complement existing native retirement tests, not unrestricted native safety
proof. Windows native execution remains unavailable under T1266/T1271.
