# T1100: process-owned broker authority admission

Parent revision `a40ed77d`. The native authority worker contract requires a
reused process admission domain, but broker startup previously constructed a
fresh capacity-one pool on every attempt. A retained old initializer, transaction,
destructor or join could therefore coexist with another newly admitted worker.

Normal `Server::open` now uses one process-owned cached pool. The startup-only
`BLENT_USB_AUTHORITY_WORKERS` setting accepts 1–4096, default 1. Startup validates
settings, reserves an original admission, then performs directory/socket effects
and transfers that admission into `Worker::start_reserved`. The worker retains
its exact thread in the existing retirement observer. Explicit isolated backend
test contexts remain separate from this process default; existing tests no longer
accidentally compete for an unrelated default pool. Only process-default fixtures
share a test serialization guard. No production reset or missing-owner fallback.

## Permanent regression and validation

`t1100_default_start_rejects_process_authority_exhaustion_before_effects` reserves
all process capacity, then invokes the real default broker startup. It fails on
the original fresh-pool implementation: startup succeeds despite exhaustion.
Any unexpected server is actually shut down/dropped before the assertion.
The same test passes after the fix and verifies no runtime/state directory was
created on rejected admission. The initial command without the required
`accessory-transport` feature ran zero tests; that log is explicitly retained as
an invalid experiment configuration, not red/green proof.

The actual-thread withdrawal test holds initialization and then a destructor
through a partial-initialization failure. In both stages, dropping the worker
does not complete the original admission: the same backend context rejects
startup before filesystem effects. Only after both stages return and the
retirement observer joins the exact original thread does capacity become reusable.
Its cleanup sponsor releases both stages and waits for actual completion on
every exit before the containing private context can disappear. This exercises
the owned-worker drop used when an opening future is withdrawn; it does not
simulate an actual stuck filesystem/provider syscall.

The cache test preserves original pool identity and failed initialization;
neither can be replaced while retained obligations may exist. Existing authority
tests retain closed-receiver, initialization-error, blocked-persistence, exact
job return and actual-thread retirement coverage.

Fresh, exclusively owned `cargo-llvm-cov` collection: **212 broker tests passed,
zero failures/ignored**, with the required accessory feature. All **25 maintained
production functions** in the scoped files pass the individual 80% executable-line
gate; minimum 83.33%. Raw LCOV, manifest, source/object hashes, command attestation,
per-function results and original logs are retained. Linux private sockets,
synthetic helpers and TLS/media fixtures were used; no physical USB operation.
Rustc 1.98.1 and cargo-llvm-cov 0.8.7 ran from installed offline tools.

The exact repository Rust source audit with cached tree-sitter-rust 0.24.2 and
tree-sitter 0.25.2 measures 115 functions, maximum 8. These are Sonar-style source
estimates, not native SonarQube evidence; T1019 remains blocked. Twelve
architecture checks pass. No thresholds, exclusions or tests were weakened.

## Ownership and bounds

The process sponsors one cached pool and its retirement dispatcher for process
lifetime. Each admitted startup owns exactly one reservation before platform
effects. Rejection owns no native worker; a no-start failure drops that unused
reservation after owned local cleanup. Once a thread starts, the reservation
moves with its exact thread/observer, including initializer failure, lost
receiver, cancellation, blocked destruction and join. Neither logical completion
nor a reference count releases it. Existing native thread completion/destruction
is observed before admission settlement and reuse.

Capacity exhaustion rejects attempts; it does not allocate a replacement pool.
Successful cached initialization has a single owner; failed initialization stays
visible until process restart. Scoped test domains own their own fixed pools and
retain their original records through actual retirement. Conditional eventual
cleanup assumes the original worker and its destructor return, the reaper is
scheduled and the actual join completes. Stuck obligations remain owned/funded.

No buffer loan, unchecked cast or new storage implementation is introduced.
Two one-shot channels bound the thread fixture's stages, with an explicit cleanup
sponsor and no elapsed kill deadline. Runtime identity-store ownership in link
tasks is a separate unresolved T1102 prerequisite; configurable cleanup budgets
remain T1101 and joined preference acceptance remains T898. This scope establishes
neither native Windows support nor unrestricted native lifecycle safety.
