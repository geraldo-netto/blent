# T1229: expose lower camera output profiles

The existing profile schema accepts320×180 and640×360, but the GUI omitted them.
Both sizes are now selectable. Defaults, permission, native ownership and manual
Start stay unchanged. The permanent rendered-UI regression first failed because
640×360 was absent. It now selects and saves both profiles, validates them and
proves no camera backend creation or display preference change occurred.

The final normal camera GUI cohort passes9 tests. A fresh LLVM target measures
every executable line of the changed resolution function at100%; raw LCOV,
executed object/source identities and red/green logs are retained. Rustc1.98.1,
cargo-llvm-cov0.8.7. Source complexity estimate5; native SonarQube unavailable.
No tool download or network transfer occurred.

This function borrows the GUI-owned CameraProfile for the immediate frame.
Its fixed five-choice list stores no native resources, callbacks or queued work.
Selection changes only the existing width/height fields; Save and explicit Start
retain their current owners. The temporary menu borrow ends with its frame.
The rendered selection/save regression retains the original isolated config
directory until asynchronous Save actually completes.

Actual camera metadata confirms native640×360 on the current tablet but excludes
320×180. T1231 separately implements explicit180p export from360p capture, with
native bounds preserved. T858 retains physical media qualification. This commit
establishes profile selection and persistence, not unsupported native capture.
