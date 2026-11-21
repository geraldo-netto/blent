# T1473: shared USB broker placement, 2026-11-21

The elected broker intentionally outlives the requesting GUI/display daemon.
Direct launch inherited the daemon service group, preventing later service start.
With a local user manager, move into an independent systemd scope before exec.
Without a user manager, retain direct native launch. Once a manager is selected,
startup failure never falls back into the caller's service group. Paths remain
separate argv values, preserving AppImage/native siblings and spaces.

The native placement regression failed with identical parent/child cgroups before
the fix. All 249 selected broker tests passed afterward, including competing
clients, owner election, starter admission and delayed native retirement. The
three changed production functions have 100% individual line coverage. Measured
production/test functions <=9 using project Rust syn 2.0.119; rustc 1.100.0 and
cargo-llvm-cov 0.8.7 supplied native counters.

Installed daemon 543768 exited normally in 0.323 seconds and restarted as 546310.
Broker 543835 remained in its independent run-*.scope; shared ADB 3379 also stayed
alive. Display render acknowledgements resumed. No broker termination was needed
for that stop/start. Scope and service retain KillMode=mixed, SendSIGKILL=no and
TimeoutStopSec=infinity. This is native acceptance on one Linux desktop, not
acceptance of unavailable platform backends or unrestricted leak freedom.

Ownership: Launch owns four absolute paths; the command copies bounded argv for
one native starter. Existing semaphore admission survives caller cancellation
until original spawn returns; spawn_reaped retains its exact child. The elected
broker owns its socket, sessions and native helper obligations independently of
client completion. Scoping changes placement, never socket identity or completion
proof. Failed contenders keep the existing owner election and reaping path;
empty completed scopes were observed to be collected after fixture execution.
