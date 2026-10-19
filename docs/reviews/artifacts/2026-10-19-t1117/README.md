# T1117: retain original request identities through failed cleanup

See ../../2026-10-19-idle-retirement.md for native ownership, positive namespace
settlement, numeric-cookie compatibility and conditional cleanup assumptions.
Both permanent regressions fail before fixes, then pass. The unchanged native C
parser independently accepts padded cookies under ASan/UBSan/LeakSanitizer.

All 30 normal idle tests pass. The bounded model covers 257 failure-count traces,
33153 attempts, and exactly one positive settlement per trace. Native original
worker completion remains pending through unreadable storage and completes only
after repair and actual cleanup. The single-attempt faulty control is rejected.
Original source bytes match the native coverage manifest after restoration.

Fresh offline LLVM counters cover all 32 production functions in the idle and
directory modules individually at 90.9–100%. Source estimates cover 76 Rust
functions (maximum 7), Python fixture (1), and C fixture (2); no native SonarQube
claim (T1019). Five architecture tests pass. This evidence predates T1118 staging cleanup;
replacement-source stale-advice admission remains T1115. No device/network work.
