# T862 — retained acceptance evidence audit

A generic historical USB-open failure lacks enough evidence to identify its cause. Existing fake-libusb tests passed, but later successful lifetimes cannot retrospectively supply that missing native status.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The complete retained attempt3 log is one line: “USB probe failed: cannot open
re-enumerating target”. It contains no native open status, event timestamp or
contemporaneous node-permission timeline. Later recovery and successful attempt5
cannot supply the missing cause; their observations concern different lifetimes.
The live provenance identifies the later source/executable snapshot, not a
recoverable native status for attempt3. Do not infer permission denial, device
replacement, busy interface or a new retry policy from this generic failure.

Current host/accessory/accessory.c records nonretryable re-enumeration status,
retains the last transient status, retries only absent/ACCESS/NO_DEVICE within
100 attempts and preserves strict initial/nontransient failure. Seven existing
AoaProbeEnumerationTest cases pass with offline ASan/UBSan fake-libusb fixtures,
including transient recovery, bounded persistent failure, strict errors and exact
selected metadata. These fixtures do not establish the historical cause or a
physical re-enumeration result. No production behavior or test changed.

T1173 separates missing historical attribution from T1174's joined native campaign.
T1173 needs a contemporaneous diagnostic or an owner-approved matched reproduction
with numeric status and identity/permission timeline; a new result must remain
distinct from retrospective attribution. Any confirmed fix then needs its permanent
failing-before/passing-after regression and source-matched individual counters.
T1174 still needs postponed T855/T858–T860 production-device qualification and an
owner-approved physical window, plus the existing automated fault/isolation gates.
T862 remains blocked until its acceptance conditions are met. No tablet, USB,
service, suspend, lock, install, network transfer or push action occurred.

</details>
