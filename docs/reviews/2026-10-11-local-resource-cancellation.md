# T994: delayed local Stop preserves the shared connection

A local speaker EOF initiated retirement while the independent control route was
also stopping. Remote confirmation and actual local pump retirement could finish
before a queued Cancel reached the link reactor. Strict Session::cancel then
returned `resource not registered`, and the reactor treated it as a link failure.
This could interrupt sibling resources. The joined private mutual-TLS/PCM test
reproduced this race six times in eight bounded trials.

The local command relay now checks the entire original lease against its current
registered binding. It cancels pending or admitted original bindings, remains
idempotent during closing, and ignores obsolete requests after retirement or
replacement. Strict wire and native-completion callbacks retain their existing
stale-input errors. There is no tombstone collection, history growth, implicit
reconnect, new native owner or inferred retirement from cancellation.

Permanent deterministic regressions failed before the fix. The same tests pass
and exercise sibling media delivery, exact replacement preservation, each full
lease identity component, pending consent and current cancellation. The joined
private speaker route passes eight further bounded trials. No tablet trial or
external dependency change was needed.

[Native collection, regressions and individual counters](artifacts/2026-10-11-t994/README.md).
