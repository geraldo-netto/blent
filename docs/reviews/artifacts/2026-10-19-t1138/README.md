# T1138 variable scope cleanup

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Rename the observer slot candidate/nested proof bindings, producer-owned options
and USB listener creator. Test factories now use descriptive names so local
reports/options retain unambiguous identities. These are mechanical binding
renames; admission, actual completion, resources and regression assertions stay
the same. No new allocation, handoff, release or native access is introduced.
Existing T1124 exhaustive transition/capacity tests and T1126 native
creation/cancellation/invalid-credential fixtures remain in the normal suite.

Fresh source/compiled-object-attested offline native evidence is under
`/backups/disk2/blent-work/2026-10-19-open-items/t1138/native-{common,host}`.
The common observation cohort passes all three tests; all seven production
functions have 100% executable-line coverage. Host results are recorded below.
The pinned tree-sitter source audit remains <=9 and is not actual SonarQube
measurement (T1019). No physical device, Windows/EVDI or network action.

Host suite: test result: ok. 924 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 74.56s
All 15 camera/USB production functions pass individually >=80%, with no new
exclusions. Source audit: 72 functions, maximum7; Cargo formatting passes.
