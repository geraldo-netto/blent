# T877 local broker control contract

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Add typed control/report operations to authenticated local packet IPC. Device
selection names an inventory revision and opaque target; approval names an exact
connection and pairing ticket. No native path, PID, USB handle or arbitrary remote
address appears in the public contract. Snapshot records carry revisions and end
markers. Commands and notices reject malformed extents, flags, counters, peers,
state/ticket inconsistencies and remembered denials; no descriptor is permitted.

Offline validation passes 70 host broker tests and 337 common tests, including
five new bounded codec/property tests and one native authenticated IPC round trip.
All 34 scoped production functions pass native per-function 80% LLVM coverage;
44 reviewed production/test functions have cyclomatic complexity at most 9.
Counters, source inventory, logs and hashes are retained. This defines and tests
contracts only: T853 owns selection/approval enforcement and bounded publication,
T855 owns production presentation. No download or live connection changes.
