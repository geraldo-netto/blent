# T1465: shared ADB placement, 2026-11-21

Both shipped service templates bootstrap ADB in an independent user scope before
starting the daemon. The same stable AppImage/native launcher selects ADB through
its normal PATH. Daemon clients use explicit loopback tcp:127.0.0.1:5037, which
does not implicitly launch a replacement when the server is absent. A missing ADB
executable leaves direct USB available; other native failures remain errors.
Server loss is reported as unavailable until a separately started server returns
or Blent restarts. No shared-server stop, PID adoption or global mapping removal.

The template regression and native private-service placement regression failed
with the original units; the optional-ADB regression failed before its guard.
All 3 bootstrap unit tests, 3 selected entry/DPMS tests and 15 selected service/
installer tests passed. The final isolated ADB tests also passed 3/3: two complete
fake-server lifetimes (each with Blent restart), preserving the same fake server
through stop/restart, plus a real ADB client against a reserved unused loopback
port. The fake server has no USB/TCP access. Native real-server PID 3379 and its
independent scope survived installed daemon stop/restart; the final tablet again
acknowledged rendered frames. T1473 separately fixes shared USB broker placement.

Changed Rust functions dispatch/start/main each have 100% executable-line coverage;
all measured functions <=9 with the project Rust syn 2.0.119/Python/shell checker.
Counters: rustc 1.100.0, cargo-llvm-cov 0.8.7. Service writer top-level branches are
exercised by both template renderings; its existing quoting helper is unchanged.

Ownership: systemd admits the bootstrap into its own scope before native ADB
execution. That scope owns any forked shared server. The bootstrap waits for its
original client and creates no output collector or asynchronous retained state.
Missing executable admits no child; nonzero/other OS errors are surfaced. Blent's
existing route/creator/cleanup fences and service retirement policy are unchanged.
Non-systemd direct invocations retain their existing launch semantics. These
bounded tests and one real tablet do not prove unrestricted native leak freedom.

Upstream startup semantics inspected in AOSP client/adb_client.cpp and
socket_spec.cpp from https://android.googlesource.com/platform/packages/modules/adb/
(native test uses installed ADB 34.0.4-debian). Explicit numeric loopback is a
connect-only client endpoint; server bootstrap explicitly uses tcp:5037.
