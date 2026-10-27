# T1114: coordinate the original idle namespace

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

See ../../2026-10-19-idle-namespace.md for the exact two reproduced bugs,
original directory/source/token ownership, synchronous callback bounds and private
namespace assumptions. This T1114 evidence predates the separate failed-removal
fence subsequently resolved by T1117 and staging cleanup resolved by T1118.

Both snapshot-clobber and renamed-directory regressions fail before the fix.
The intermediate initial-green/green logs expose a fixture permission mismatch:
installed tempfile modes follow umask. Fixtures now explicitly allocate private
roots; no original assertion is removed or weakened. All 25 normal idle tests
pass, including native independent-description interleavings, actual process
lock exclusion, blocked-lock Stop and nonescaping failed-persist cleanup.

Fresh offline LLVM counters measure all 27 production functions individually
at 90.9–100% executable-line coverage. Source estimates cover 66 functions,
maximum 7; native SonarQube remains T1019. Five architecture tests pass.
Removing cleanup coordination or the post-lock Stop check is rejected by the
permanent regressions. Correct source bytes match the native coverage manifest
before final normal verification. No physical or network action occurred.
