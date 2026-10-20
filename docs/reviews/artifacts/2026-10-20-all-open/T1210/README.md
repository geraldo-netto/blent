# T1210: current USB source-output fixtures

A new permanent normal-suite control first fails because the Cargo fixture does
not generate the current broker. The fixture now emits current Rust executables
while explicitly leaving native USB I/O to its C build owner. Disposable source
builds copy the current USB Make inputs and their compiler fixture writes the
actual selected output. Stale/source/prebuilt/tar oracles include both USB
artifacts; source paths, arguments and selected Cargo directories remain literal.
Existing XDG/service/autostart/rollback assertions remain unchanged.

All7 source-output and5 inherited user-path cases pass after T1213's independent
native compiler/freshness fix. Retained red/core-oracle failures are source input
failures, not permission to relax native device contracts. These are bounded
normal-suite fixture changes, exempt from production coverage; source estimates
for every changed/new fixture function <=9. Raw logs/hashes retained; unavailable
native Sonar remains T1019. Temporary-directory owners retain their generated
programs/logs and subprocess handles until test settlement, then clean up. No
native device, running service, EVDI implementation or network action.
