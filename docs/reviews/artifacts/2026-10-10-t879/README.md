# T879 selected USB metadata gate

Selected USB setup now rechecks descriptor metadata before any mode-switch command. The replacement regression rejects with no control transfer; metadata still is not authenticated identity.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The production-selection entrypoint accepts `--selected BUS PORT.PATH ADDRESS
VENDOR PRODUCT`. Rust passes bounded decimal arguments without a shell. The
helper opens the physical target and compares its observed address, VID and PID
before any AOA control transfer. A mismatch fails closed and requires a fresh
inventory selection. Expected AOA re-enumeration may change address and PID;
the initial check does not incorrectly prevent that transition.

These descriptor observations bind a selection, not an authenticated identity.
Only the later fresh TLS proof grants pairing authority. Identical descriptor
values are not a claim of unique physical identity. Existing explicit physical
target probe entrypoints remain available; T853 uses the selected entrypoint.

The permanent replacement regression first failed: mismatched metadata produced
eight control transfers and successfully switched the fixture. It now rejects
with zero control transfers. Native tests also retain re-enumeration recovery,
streaming, read-only discovery, bounded parser mutation and invalid extents.

Offline validation: 21 Python tests pass with ASan/UBSan native fixtures; ten
Rust accessory tests pass. Per-function native counters pass for all 31 C and
seven Rust production functions in the recorded scopes. The minimum is 87.5%.
All 102 reviewed production/test functions have cyclomatic complexity at most9.
The real helper builds against cached stock libusb headers and runtime library.
It was not run on live USB devices. No downloads or installations.

Counters, source hashes, red/green logs and coverage manifest are retained here.
The coverage snapshot includes concurrent uncommitted T853 integration work;
only the four source scopes named in coverage.json are claimed by this item.
Production application composition and device acceptance remain T853 onward.

</details>
