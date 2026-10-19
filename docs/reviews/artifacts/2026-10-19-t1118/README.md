# T1118: retain exact staging ownership through cleanup and owner loss

See ../../2026-10-19-idle-staging.md for directory-borrow binding, native file
identity/cleanup, reserved namespace admission and conditional recovery limits.
Destructor-only cleanup first fails both permanent unlink-failure and foreign
replacement regressions; the absent-recovery prototype also fails owner-loss
collection. All pass with retained native ownership and guarded recovery.

All 39 normal idle tests pass, including actual child exit without Rust stack
destructors, recovery after original native completion, bounded mixed namespace
traversal, rejected untrusted objects and partial/persist-error ownership.
A deliberately removed identity check is rejected. Restored source bytes match
the fresh native coverage manifest before final normal verification.

All 45 production functions in idle/directory/staging individually measure
90.9–100% executable-line coverage. Source estimates cover 98 Rust functions,
maximum 7; native SonarQube remains T1019. Five architecture tests pass and the
normal executable checks without warnings. No device or network action occurred.
