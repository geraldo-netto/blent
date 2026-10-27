# T934 exact camera consent ownership

Camera consent requires the original prompt, current local intent, fresh OS permission and actual protocol admission. Closing a prompt revokes intent without proving native retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

A portable policy owns one offered camera lease and issues one retained prompt.
Preparation requires current local intent, current OS permission and the exact
original runtime's offer authority. Capture additionally requires actual protocol
admission. Prompt object identity prevents replay into an equal-counter replacement.
A permission callback must still match the retained prompt, live authority, local
intent and foreground permission context; its boolean grant alone is insufficient.

Foreground/background decisions remain independent: background capture requires
both the host profile and local allowance. Local stop/denial, permission loss and
connection loss explicitly emit exact rejection/cancellation. Closing clears the
prompt immediately but never asserts actual native retirement or permits replacement.
Native composition must retain the policy and any partial endpoint ownership until
that independent proof. T935/T936 provide local/IPC adapters; isolated camera capture
and capability enablement remain T932, with joined native acceptance T858. Production
capabilities remain display-only.

All 639 offline connection tests pass, including ten permanent consent tests for
permission denial/unproven grant, opaque stale/reconstructed prompts, equal-counter
replacement, independent admission, background/local intent, authority loss, stop,
permission revocation, preparation revalidation and bounded invalid/range/property
inputs. Every camera profile parameter has invalid boundary cases; 128 seeded
stale/invalid repetitions remain bounded. This is a new policy, not a claim that
production camera support or a pre-existing camera vulnerability was fixed.

All 15 authored inventory methods have 100% individual native JaCoCo line coverage;
seven authored primary constructors also have 100%, supplementing T848's existing
inventory gap. Compiler-generated default-constructor bridges are distinct from
the authored primary body and are not authored methods. All 34 reviewed methods
have cyclomatic complexity at most nine. XML/execution counters, source inventory
and hashes are retained. These are JVM/Robolectric results; physical trials remain
owner-postponed. No downloads, installs, reloads or USB mode changes occurred.

</details>
