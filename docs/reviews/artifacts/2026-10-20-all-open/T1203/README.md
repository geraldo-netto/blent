# T1203: remove obsolete V4L2 probe CLI and narrow decoder requests

The actual Options parser was extracted unchanged before behavior repair. The
permanent old-flag rejection test fails first: --front-device is accepted despite
the probe opening no V4L2 outputs. CameraProfile now defines the input contract;
--front-device/--rear-device are rejected without compatibility aliases. Current
camera/measurement arguments still parse and feed the actual bounded timestamp
store. No unused native CameraOptions wrapper is constructed for the probe.

The decoder's filters/command/connection/transfer interfaces now borrow only the
portable CameraProfile. All current production and test callers explicitly pass
their profile view. Native output-device paths stay with their real Linux adapter;
no legacy decoder signature or overload is retained. Profiles/header rotations
are validated at current entry boundaries before decoder effects; header fuzz/
invalid-input regressions remain. CLI, camera wire and native lifetime behavior
otherwise retain their current contracts.

All 100 targeted cases pass: 98 native camera/protocol/pool/ownership tests and
2 actual parser/store-boundary cases. Current transformation, FFmpeg transport,
error, stale/retained native process and producer retirement regressions remain.
Native LLVM counters meet the individual threshold for all five changed
production functions: filters/command/connection/transfer 100%, sessions 90.48%.
Exact source ranges, raw counters, hashes and commands are retained. Export uses
only current library/parser executables. Benchmark parser/caller functions keep
the explicit coverage exemption. Manual source decision bounds are <=9 (decoder
filters <=6, transfer <=8, sessions <=3); native SonarQube/grammar remains T1019.
Formatting/whitespace checks pass. The pre-existing probe Report integration and
full example compile acceptance still belong to T1189; this is not a full probe,
workspace or physical camera acceptance result.

CameraOptions still owns actual native output configuration where needed. The
borrowed CameraProfile view cannot outlive its owning request/future and carries
no device/OS handle. Narrowing removes unnecessary representation dependence; it
adds no allocation, duplicated mutable state or new native release/reuse action.
Decoder child admission, original Report reservation/bind, OwnedChild retirement,
frame-pool Arc leases and exact caller context remain authoritative. Existing
frame readers retain immutable buffers; pool reuse still requires exclusive
ownership. Profile/header bounds and current native process failure/cancellation
oracles pass. Numeric/logical completion is not substituted for native evidence.
Conditional cleanup still requires original native exit and reaper/scheduler
progress; hosted cases do not prove unrestricted safety. No live device/service,
Windows/EVDI backend or network transfer was changed.
