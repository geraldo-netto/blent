# T994: obsolete local resource cancellation is harmless

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

See [the diagnosis and contract](../../2026-10-11-local-resource-cancellation.md).
Two deterministic permanent regressions failed before the fix; all three pass
after it. Strict Session::cancel and native callbacks remain unchanged. Tests
cover exact full-lease identity, pending and current cancellation, closing,
actual retired resources, replacement preservation and sibling media delivery.

Fresh offline native broker collection: 200 tests pass, including private real
mutual-TLS speaker routing. All 31 production functions in the affected link and
session scopes pass individual 80% executable-line coverage with no missing
counters. Changed Rust complexity: 34 functions, maximum eight. Eight bounded
joined speaker trials also pass after the fix; the earlier diagnostic reproduced
the race six times in eight trials. Compressed red/green logs, LCOV, counters,
source/native inventories and attestation retain the evidence. Collector used
an exclusively owned target and immutable inputs. No downloads or live trials.

The build-input attestation describes the working tree, including pending T992
speaker implementation/fixtures and pre-existing uncommitted TLS fixtures.
Those unrelated/pending edits are not included in this commit. The deterministic
T994 regressions are independent of that pending speaker implementation.
