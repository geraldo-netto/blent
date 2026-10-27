# T1077: library counters do not attribute authored inline bodies

Library coverage counters cannot stand in for an authored inline callback's missing execution evidence. The stricter checker rejects the retained false pass while preserving all 39 independently measured T1061 functions.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The permanent fully counted library-class fault control fails before this fix:
Emitters.kt counters let a same-line authored transform disappear from the gate.
Native inline-owner associations now remain visible even when all library methods
have counters. An unattributed authored body makes its enclosing method unmeasured;
actual owned callback counters resolve it. Library-only wrappers without authored
callbacks and genuine inline code measured within its enclosing method retain
the existing policy. No new source exclusion or fabricated counter is introduced.

All 34 report/import/driver/callback/invocation checks pass offline. The retained
T1069 counter gap is still rejected; all 39 measured T1061 functions/callbacks
still pass. Raw native XML/class/SMAP evidence stays in the original artifacts.
This is conservative compiled-owner attribution, not unrestricted SMAP proof:
missing attribution needs measurable authored code or actual native evidence.

The pure report projections retain the existing bounded read/caller-owned data
lifetime and introduce no native resource or asynchronous cleanup owner. Python
3.12.3 AST source audit covers 40 functions, maximum estimate 9. Native SonarQube
remains T1019; the existing nonessential-tool coverage exemption is unchanged.
No download, network transfer, application behavior or test threshold changed.

</details>
