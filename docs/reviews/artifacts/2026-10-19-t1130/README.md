# T1130 portable connection lint maintenance

Portable connection lint changes preserve borrowed frame lifetime and input bounds. Focused tests passed; a serial retry did not fix the separately recorded timing failures.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Rename the lending Multiplexer method to `next_frame`, updating current native,
probe and test callers. Frame bytes remain borrowed from the same original
writer reservation until finish; no Iterator semantics or new copy is introduced.
Use unsigned divisibility methods, fixed-size trust-record iteration and direct
Copy profile transfer. Bounds/header/count validation still precedes every slice
and conversion: trust records retain the 88-byte header, 64-byte peers and
1024-peer maximum. Malformed, truncated, noncanonical and oversized records are
still rejected. This change creates no resource owner or escaping borrow.

Installed Clippy1.98 reproduces seven errors before, one after: the remaining
EVDI-specific capture-control diagnostic is explicitly deferred T1131. No lint
suppression or dependency update. Existing normal common library tests383 and
integration cohorts5/8/5/1 pass; the host accessory broker cohort passes229.
The accessory probe compiles offline; it is not run against any device.

The first fresh instrumented common run failed two unchanged timing fixtures;
its exact log is retained, and T1139 remains open. A fresh serial collection
passes all those tests unchanged. A retry is not claimed as a timing fix.
Source/compiled-object-attested native evidence is in
`/backups/disk2/blent-work/2026-10-19-open-items/t1130/native-common-serial`
and `native-host`. Host counters were re-exported from the same attested objects
and profiles with both workspace packages selected, so dependent common code
is visible; `extra-export.json` records hashes and command. No test rerun or
source mutation was used for that export.

The combined individual coverage gate passes47/47 production functions in the
four changed common files and host writer file, >=80%, none unmeasured or newly
excluded. The pinned source audit checks139 production/test/example functions,
maximum9; it is not actual SonarQube evidence (T1019). Cargo formatting passes.
No Windows/EVDI edit, install, physical-device action or network transfer.

</details>
