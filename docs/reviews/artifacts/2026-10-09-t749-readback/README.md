# T749 — isolate the virtual GL readback failure

**Partial progress; T749 remains unresolved.** The cached Weston13/virgl guest
renders the expected pattern correctly. Making selected GBM rows CPU-readable
before scanout makes exactly those rows available to EVDI. The failure is in
this virtual GPU export/import/CPU-visibility path, before Blent conversion;
it is not evidence of a Blent NV12 bug or a generic EVDI snapshot-order race.
The tests do not identify which upstream component owns the missing transfer.

A retained **guest-only diagnostic**, `gbm_readback.c`, wraps
`gbm_surface_lock_front_buffer` and calls the unchanged original. `SNAP_READBACK`
selects one of three controls. Stock Weston, Mesa/virgl, EVDI1.15.1 and
libevdi1.15.0 are unmodified. Same immutable SHM pattern client, helper and
independent raw/NV12 validators as the earlier T750 experiment.

| Control | Recognized raw IDs | Full-frame raw result | NV12 result |
| --- | --- | --- | --- |
| `wait`: 2 ms delay, no map | none | no recognizable pattern | fail, no valid frames |
| `strip`: map only top 16 rows | all 0–240 | all 244 recognized captures fail at byte-oracle position 10241 | all 281 frames fail |
| `full`: map all 480 rows | all 0–240 | all 247 recognized captures correct | 284 valid frames, no backward IDs |
| `full`, repeated unchanged | all 0–240 | all 243 recognized captures correct | 281 valid frames, no backward IDs |

The oracle position is one plus the pixel index: **10241 is x=0, y=16**, the
first row outside the strip mapping. Thus timing alone does not explain the
result. Top-row identifiers and their complements are correct, while untouched
rows remain unavailable. Full readback validates every RGB channel/pixel in the
GBM buffer before import, and the independent observer validates the pixels
returned by stock libevdi afterward. An initial full-map prototype also passed
(243 raw checks, 281 NV12 frames); its exact source is retained separately.
Counts include repeated final snapshots/keepalives, not extra source commits.

Mapping is an **active diagnostic intervention**, not passive observation:
it can trigger renderer transfers and synchronization. Full-map success therefore
does not certify the original unassisted GL path or close T750's ordering
acceptance. The partial-map and delay controls intentionally fail the unchanged
consumer; no corruption assertion was skipped or relaxed. Genuine black
background before/after the client is distinct from recognized frame validation;
no recognizable pattern always fails. All runs reach 241 presentation callbacks,
whose reported timestamps/sequences remain zero as in the earlier experiment.

## Why no production mitigation was added

Blent's stock EVDI userspace API receives copied pixels and dirty rectangles. It
has no compositor-owned GBM object/exporter handle with which to request this
readback. Loading this diagnostic into a desktop compositor would change an
external process and add a synchronous full-frame readback/copy cost. It is not
an appropriate general Blent fallback. A black desktop is legitimate content,
so blank-frame rejection is not a valid substitute. Keep stock dependencies and
existing bounded recovery; do not add double-grab or stale-pixel heuristics.

## Remaining acceptance prerequisites

1. A **stock exporter/reproducer that actually supplies unequal DMA-BUF-size
   and SG-derived page counts**, plus permanent automated coverage of that
   boundary. Ordinary successful imports or a userspace copy of the upstream
   allocation formula would not exercise the real branch. Stock Blent APIs
   neither expose nor select the kernel SG page layout. No such fixture is
   available in the cached environment; no bespoke kernel exporter or EVDI
   patch was introduced to manufacture one.
2. Native acceptance of a distro-compatible, released stock module on the
   physical target desktop, then rollout. The existing module build result
   and CPU-copy reconnect success remain useful but insufficient. These
   virgl/guest results do not establish physical AMDGPU/Xe behavior.
3. T750 needs the frame-identity/damage-order test on that working **unassisted**
   GL/import path. Explicit GBM readback adds synchronization and cannot satisfy
   this prerequisite by itself.

Read-only host check: running kernel `7.0.0-38-generic`, package
`linux-main-modules-evdi-7.0.0-38-generic` version `7.0.0-38.38~24.04.4`;
loaded/on-disk module srcversion both `C477D84009DFBE1A8FCE08C`. No package
installation, module reload, service restart or desktop configuration change.
No downloads or network requests. The guest module has kernel 7.0.0-34 ABI;
previous target-header build evidence remains historical, not target execution.

## Reproduction and verification

```
cc -O2 -shared -fPIC scripts/benchmarks/evdi_snapshot/gbm_readback.c -lgbm -ldl -o snapshot-gbm.so
```

Run only in a disposable VM marked `blent-evdi-review=1`. Start the normal helper,
raw observer and strict consumer. Apply `SNAP_READBACK=wait|strip|full` and
`LD_PRELOAD=/snapshot-gbm.so` **only to Weston**, using the retained guest scripts
and the earlier T750 cached-assets setup. The GL renderer uses virtio card1 and
EVDI card0; physical target acceptance is separate. No benchmark binary is shipped
or loaded by Blent. Compiled functions meet cyclomatic <=9. The unmarked-host
guard rejects with exit64 under ASan/UBSan before any GBM operation.

The existing normal-suite T750 coalesced-snapshot regression passes under its
ASan/UBSan harness (`owned-regression.log`). There is no new production C behavior
or fabricated regression for the still-missing SG mismatch. Sources, binaries,
full compressed guest logs and exact counts are retained next to this report.

The same full-readback probe was also built with
`-O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all` and run in the guest
with the cached SDK's libasan preloaded first. It produced **311 correct NV12
frames**, with no invalid-access or UB diagnostic from the owned probe. The
whole-compositor exit leak check is **not clean**: LeakSanitizer reports 10,267
bytes in 89 allocations, with retained stacks in external Weston startup,
configuration and udev ownership, none in the probe. No suppression, external
patch or external audit was added; this is a limitation of a process-wide
sanitizer measurement, not a claim of clean third-party code. The full log and
separate result are retained in `t749-sanitized.log.gz` and `summary.json`.
