# T1102: identity consumers retain original store ownership

Parent revision `eb147ef1`. The authority initializer previously published a
key Arc separately from its native identity-store guard. Link settings retained
only the key. Dropping the provider/server could therefore unlock the store while
queued link work still held the old identity. The permanent real Linux-store
regression fails before the fix and passes afterwards with unchanged assertions.

`RetainedIdentity` now requires an explicit opaque, acyclic resource guard before
publication. Linux supplies its original `LinuxIdentityStore`; pure memory test
providers explicitly supply a unit guard. The guard follows the original parsed
signing key, every cloned TLS configuration/certified key and each selected
signer. There is no missing-owner fallback. Link callers still receive the narrow
identity view and inherit its owner without a second mutable owner field.

Identity loading uses stock rustls 0.23.45 `CertifiedKey::from_der`, the same
parse/key-match operation used by its previous single-certificate builders.
TLS builders use stock `SingleCertAndKey` resolvers to retain that parsed key.
Exact peer checking, protocol negotiation, ALPN, TLS versions and disabled
resumption/tickets remain covered by existing authentication regressions.

## Permanent evidence

Six T1102 cases cover real exclusive store reopening, actual queued link startup
before its first poll, server/provider loss while an original native creator is
blocked, cancellation with native admission still funded, client/server TLS
configuration retention, independently selected signer retention, unsupported
schemes, optional metadata and failed signing. The blocked creator reads the
same original certificate after its release. A retired async task cannot unlock
the store or release its native capacity; the original creator must return.

Fixtures sponsor their original releases, tasks, authority completion and private
storage on every exit. The blocking fixture uses a multithreaded runtime so its
cleanup sponsor can observe actual task/authority retirement while unwinding.
The queued fixture destroys its runtime, then waits for the original authority
thread's actual joined completion before deleting its containing storage. No
elapsed process kill or replacement thread is used to manufacture retirement.

Fresh exclusively owned native coverage uses installed rustc 1.98.1 and
cargo-llvm-cov 0.8.7, offline, with `accessory-transport`. All **887 tests passed**, with three existing opt-in benchmarks ignored. All
**34 scoped production functions** pass their individual 80% gate; minimum
94.74%. All 94 source estimates are at most 7.
Per-function results and exact final inputs are retained in the logs/reports. The initial
non-escalated attempt could not create private sockets (`EPERM`); this is retained
as unavailable sandbox execution, not regression proof. The earlier proper red
store regression predates the behavior change. The normal native suite keeps its
three existing opt-in benchmark exclusions; no test or threshold was weakened.
Twelve architecture checks pass. Cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2
source estimates are not native SonarQube evidence; T1019 remains unresolved.

## Ownership, bounds and limitations

One provider consumes one validated identity and its original guard before
publishing any consumer. Certificate and private-key admission retain their
4096-byte limits. The guard has no back-reference to consumers, preventing a
retaining cycle. Cloning views creates shared immutable key ownership, not a
second mutable state/store. Synchronous signing borrows its input only for the
call and returns its owned signature or error; this wrapper creates no message
queue or asynchronous native operation.

Key/signer fields destroy the inner crypto object before the retained guard, so
inner destruction cannot access an already released owner. A chosen signer keeps
its own original guard even after its key and configuration disappear. Rejected
scheme selection publishes no signer; partial initialization and failed trust
opening destroy already-owned values in their containing scopes. Native creator
cancellation retains the original consumer and admission until that creator
returns; reference counts or logical task completion alone are not native proof.

Conservation: every published consumer retains its original guard; the guard
cannot end while a key/configuration/signer consumer remains. Conditional cleanup
requires all consumers and original native calls/destructors eventually finish,
with the existing retirement reaper scheduled and joins completing. A stuck
consumer keeps ownership and its existing bounded admission. Store reopening is
checked against the real Linux lock, independently of reference-count assertions.
These tests cover isolated hosted Linux sockets/threads and stock TLS, not a
physical USB device, Windows runtime support or unrestricted native safety.
