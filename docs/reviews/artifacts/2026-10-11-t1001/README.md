# T1001: explicitly owned coverage cache marker

The Rust collector writes Cargo's cache-directory signature only after creating
its exclusive new evidence/target tree, before any Cargo operation. It refuses
preexisting caller output before marking or cleaning anything. Guarded Cargo
clean can now run normally; its ownership protection is never bypassed.

The permanent test invokes real offline Cargo clean on that exact owned target,
then exports real instrumented Rust counters and verifies the unrelated cached
binary remains. Before the fix it fails with the retained exact missing/invalid
CACHEDIR.TAG refusal; the same test passes afterward. All eight collector tests
pass, retaining cached-object exclusion, source immutability, symlink rejection,
failed/empty collection refusal and bounded invalid feature/selector checks.
Changed authored Python complexity: 24 functions, maximum 7. This nonessential
development collector retains the existing coverage-scope exemption; no new
production function exclusion or aggregate substitute is introduced.

A fresh exclusively owned real cargo llvm-cov collection passes all 49 shared
audio tests and native source/object attestation, without the prior preliminary
clean warning. Its matching inventory, native report, logs and collector/source
fingerprints are retained. No downloads, installations, live devices/services,
caller targets or project dependencies were touched. The attested tree includes
the unrelated pre-existing uncommitted host TLS fixture, which is not changed.
