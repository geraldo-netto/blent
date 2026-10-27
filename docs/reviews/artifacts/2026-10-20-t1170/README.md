# T1170 — definite assignment in retained native oracles

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Replace only four redundant mutable initializers with definitely assigned vals:
SavedTrustProcessTest.detected and TrustKernelLockTest.before/after/rejected.
Forward application of these four declaration substitutions reproduces the new
files exactly. Every other byte, including assertions, try/finally control flow,
protocol deadlines, original process/Binder identity checks, close ordering and
actual join/reap operations, is unchanged. Preimages and comparison hashes follow;
historical T1151/T1156 evidence remains untouched.

The original three normal-suite cases pass offline: silent maintenance on both
SDK27/34 retains ownership after its logical deadline until actual supplied Binder
death, and the independent native process probe confirms rejected JVM store opens
cannot release the original kernel lock. Fresh unit-test compilation reports none
of the four redundant-initializer warnings. The two files' source functions audit
at maximum3 using cached Kotlin PSI1.9.20; native Sonar remains T1019. No production
function changed, so no new production coverage claim or artificial behavioral
regression is introduced. Resource owners, cutoff, settlement and cleanup sponsors
remain those of the original tests. No transfer, device action or push occurred.
