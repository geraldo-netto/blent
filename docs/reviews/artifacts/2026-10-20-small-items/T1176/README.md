# T1176: checked controller admission

Controller claims reserve the next representable epoch under the original watch
channel's write lock before initialization or device effects. Exhaustion returns
None through the current control workflow, leaves the last owner unchanged and
does not notify replacement observers. No new lease or duplicated mutable epoch
is allocated on denial. The original maximum lease can still dispatch and owns
its eventual release; stale lease drops cannot release its contacts.

Five permanent regressions fail before the fix: unchecked replacement admits all
eight concurrent claimants, runs initialization, changes decoder settings, and
violates the boundary oracle and terminal ownership assertions. The same tests
pass afterward. All98 input tests pass normally and with fresh native LLVM
counters. The34-seed independent u128 oracle checks20 claims per seed (680
transitions), including0–16 and MAX−16–MAX. Changed production methods each have
100% executable-line coverage. Raw counters, commands, hashes and red/green logs
are retained. Local decision/closure estimates are <=6; unavailable native
SonarQube measurement remains T1019, not a passing certification.

Controllers retains the sole epoch and device Arc; admitted leases borrow that
owner through Arc until Drop. Claims and original release retain the existing
write/read lock ordering, so admission cannot interleave within identity-checked
release. Denial neither borrows a fresh native device nor retires the original.
All concurrent test threads are joined; every admitted lease is dropped and
strong-count assertions verify no retained test lease. Eventual native cleanup
still depends on original users dropping their leases and the unchanged sink's
release contract; numeric denial is not proof of native device retirement.
No physical device/service or Windows/EVDI backend was changed.
