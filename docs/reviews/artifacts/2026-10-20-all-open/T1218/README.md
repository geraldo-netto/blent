# T1218: metrics for current source formats

Metrics cover current source formats and retain strict parser failures. Complexity tests passed, but three coverage fixtures lacked a usable Rust grammar; that collection was not a complete pass.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Complexity and essential-script coverage now select the current language and
package-hook formats only. The removed backend has no parser, source inventory,
section adapter or compatibility entry point. The independent package-hook
boundary control measures an actual supported postinst function at complexity10;
9/10 oracles and all other supported languages remain unchanged.

All28 complexity tests pass using cached Kotlin PSI/native local grammars. The
normal105-case coverage suite has102 passes and three environment errors. Those
three source-snapshot fixtures require the pinned current Rust grammar; the only
cached native ABI14 grammar rejects actual async closures in unchanged OSK tests.
That exact fresh reproduction and missing prerequisite remain T1221. No correct
Rust syntax, source scope, test or parser rejection was altered to hide the gap.
The separately repaired native observer test T1220 passes actual LLVM assertions.
No complete105-case pass or native Sonar result is claimed.

This removes adapter branches without introducing state or a native owner.
Inventory functions borrow the current repository source during one synchronous
collection and yield immutable function/range records to its caller. Existing
source hashing, parse/error rejection, input/report capacities and report output
ownership remain intact. Removed paths allocate/admit nothing. New package-hook
fixture owns one TempDir and actual source file until measurement returns; it
contains one bounded synthetic10-complexity function and cleans via context exit.
Source estimates remain<=9 in all changed functions (check.py<=5, inventory.py<=8,
metric tests<=4), using unchanged Python3.12 source rules; native Sonar remains
unavailable. These nonessential verification scripts/test fixtures retain their
existing production line-coverage exemption. No tool download or native device,
service, EVDI/Windows implementation action.

</details>
