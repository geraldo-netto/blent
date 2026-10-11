# T848: authored Kotlin initialization coverage

Permanent PSI regressions fail before authored primary/object/companion inventory;
the attribution regression fails before missing native initialization counters are
rejected. Both pass with the normal tool changes. Multiple property/init bodies
belong to one source constructor and every native variant must pass. Secondary
construction retains shared initializer counters. Named/anonymous objects,
top-level statics, superclass arguments, enum entry arguments, nested methods,
delegated fields and 64 bounded initializer shapes retain exact ownership.
Empty implicit/constant/interface scaffolding creates no artificial methods;
native JaCoCo filters remain authoritative, including authored default variants.
No counters are deleted or manufactured to obtain a pass.

All 78 normal coverage-tool tests pass; the full owned-source complexity audit
passes (13,536 functions, none above 9). Native attribution is checked against the
exact SHA-attested T944 XML/exec and unchanged application sources: all 121 scoped
production functions pass, including ConnectionFlow/Queue construction, default
constructor variants, process singleton initialization and enum initialization.
The native-provenance file identifies those retained artifacts. Reusing these
unchanged native counters validates the new gate, not a new device run.

The additional complete Android suite runs 1,761 tests with two API-27/34 failures
in RemoteAdaptersTest's existing remote-status fixture. T962 tracks that failure;
no whole-suite pass is claimed here. No skipped/removed assertions, lowered
coverage thresholds, platform claims, downloads, install or device action.
