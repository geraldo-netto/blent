# T789: bounded cursor ownership adoption

Bounded adoption of small native cursor allocations reduced shape-update copies while preserving replacement ownership. Large images retain normalization; unchanged cursor frames do not gain from this optimization.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Packed native BGRA/alpha cursor images up to 64 KiB now retain the allocation
transferred by stock libevdi. Blent allocates only the saved-background area.
Padded, excess-length, nonnative and larger images retain normalization into
one combined allocation. Borrowed-image callers retain their existing contract.
The ownership-taking API consumes the caller-owned image on success or failure;
failed replacement preserves the previous cursor. EVDI/libevdi are unchanged.

Five alternating trials using actual old/new cursor functions, including event
allocation/fill and final release. Median run means in microseconds:

| Square image | Copy baseline | Final bounded adoption |
|---|---:|---:|
| 16×16 | 0.176 | 0.017 |
| 32×32 | 1.025 | 0.058 |
| 64×64 | 4.035 | 0.132 |
| 128×128 | 19.965 | 0.436 |
| 256×256 | 57.369 | 57.348 |
| 1024×1024 | 959.196 | 970.389 |

Large-image differences use the same allocation/copy algorithm and are not
claimed gains. Shape-update savings do not improve every
frame: unchanged shapes already retain their image.

The permanent normal-suite T789 ownership regression failed on the baseline
allocation identity and passes after the change. It retains boundary/excess
length/stride/format, truncated input, allocation failure, replacement, hide and
background restoration coverage. Existing cursor, shared-slot and native-format
regressions remain. GCC ASan/UBSan suite and Clang ASan/UBSan T789 pass; all
210 maintained capture functions meet the individual 80% coverage gate. Changed
C functions stay within complexity 9; Rust formatting passes.

`raw.json` retains the final measurements. `red.log`, `clang.log`, `suite.log.gz` and
`coverage.json.gz` retain validation. To reproduce the benchmark offline, build
`bench.c` with `cc -O3`, including `scripts/benchmarks` and a private copy of
`host/evdi` from baseline `8aaeec0`; build again with `-DTAKE` against this
commit. Run each executable with widths 16, 32, 64, 128, 256 and 1024, alternating
order across five trials. No live capture or downloads are required.

</details>
