# T1109: bounded portable unchanged-frame policy

The implementation and independent contract, ownership boundaries, model bounds
and qualification limits are described in ../../2026-10-19-unchanged-policy.md.
Ten normal policy tests and all 380 common-library tests pass. Fresh native
LLVM counters measure each of ten new production functions at 100% executable
line coverage. The original sandbox SO_PASSCRED failures are retained alongside
the successful unrestricted local run. The source estimate covers 27 functions,
maximum 7; native SonarQube remains unavailable under T1019.

The bounded exhaustive cutoff model covers 5184 traces / 20736 transitions.
Both ignored-original-authority and delayed-cursor faulty controls are rejected;
the original policy bytes are verified against the native coverage manifest
before final green tests. No native adapter is enabled by this policy-only item.
No test was skipped, removed or weakened. No network/device action occurred.
