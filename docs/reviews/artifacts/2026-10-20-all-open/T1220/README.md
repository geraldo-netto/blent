# T1220: native retirement-counter fixture module layout

The native retirement-counter fixture compiles byte-identical production sources in their real module layout. The original LLVM assertion passes after repair, with source hashes checked before interpreting counters.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The existing permanent T841 normal-suite test fails before repair: rustc E0583
cannot locate the current observations child module through its external root
path attribute. Current test now passes with the exact same actual LLVM counter
assertion, including a positive original reap entry counter.

Compile byte-identical current production files with their original module layout
inside the owned fixture directory. Remap those exact copied source locations to
the original repository paths and reject a changed original before interpreting
its counters. No alternative observer algorithm or API is introduced. Copying
uses ordinary files/directories, without native symlink privilege requirements.
The actual current module and existing tracing stub remain compiled by rustc;
source formatting and line numbering are preserved.

The parent owns one TempDir through compilation, original child exit and LLVM
merge/export. Every native invocation synchronously settles before reuse/removal;
subprocess.run retains its existing timeout/kill/wait ownership. The deliberately
retained idle pool is bounded to one within that original child process; actual
process exit, not logical completion, settles its remaining native resources.
No production resource lifecycle changed. Local unchanged Python source model
scores both functions1 and5; native Sonar remains unavailable. Test fixture coverage
exemption applies; the strict native counter test is not removed or weakened.
Rustc1.98.1, native LLVM tools from the same installed toolchain. No device,
service, package installation or transfer.

</details>
