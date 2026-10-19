# T1137 — original camera output ownership

The camera application now starts the T1143–T1145 owned native writers on the
original validated/locked files. Output paths are never reopened. The redundant
raw-output FFmpeg children/feeders are removed; the selected camera still uses
an owned FFmpeg decoder. Decoder child retirement retains its existing shared
implementation. Final-black write errors propagate after controller draining,
even when the session itself stopped successfully.

The native worker owns its original File/Arc lease, bounded black frame and each
borrowed live-frame Arc through configuration/write return. Its controller owns
cancellation, progress observation and actual join. Report independently retains
both original completions. Admission rejection, partial startup, future/controller
loss and runtime loss cannot release pending native work. A 3 s cleanup control
budget reports failure and retires controllers; it is not proof of native stop.
Actual original return/join gates file release and replacement. Eventual cleanup
assumes native return and observer progress; permanently blocked native calls
remain owned and funded. A killed containing process has no external output writer
left running. This does not add crash containment to the separate decoder process.

Two permanent private-PID/mount-namespace PTY regressions reproduced the old
process-loss and replaced-path failures before integration. The sealed Python
oracle is unchanged: a live original writer must retain the original lease and
target; an actually dead creator/thread group must produce no later bytes; a
replacement may never receive output. Original pidfds, peer credentials, creator
relationships and actual cleanup are checked before assertions. 27 malformed
ancillary-descriptor cases retain no original lease. The declared configuration
port replaces only ioctls; native writes, file locks and task ownership are real.

A separately sealed successful-start/failed-final-black regression failed before
cleanup errors were propagated and passes unchanged afterward. The budget fault
test uses a deliberately unresponsive controller around a real kernel-lock-held
native task: timeout preserves the original file lock until independent release
and actual join. Existing cancellation/admission, final-black, EOF/flush, decoder
and foreign-client regressions remain in the normal suite. Shared frame selection
borrows a byte slice; the independent metadata parser accepts at most 128 bytes
and exactly eight unsigned 32-bit fields. Permanent bounded invalid-length, plane-edge,
field-count, UTF-8 and integer-range corpora cover those test oracles.

Two fixture failures are retained. The first released configuration before actual
controller destruction and entered a blocking private-PTY write; recovery drained
only original masters duplicated with pidfd_getfd in its private namespace. The
fixture now gates configuration on actual controller retirement plus independent
supervisor release. The added budget fixture initially assumed automatic paused
clock advancement during a blocking task; cached Tokio 1.53.1 documents that this
is disabled. Its proven wait cycle was cleaned up by exact pidfd termination of
only the owned test process, followed by reaping, fresh lock probes and private
file cleanup. Explicit clock advancement fixes the fixture without weakening its
ownership assertions. Neither recovery touched a live terminal or camera.

Final evidence: 90 default camera tests and 973 accessory-enabled host library tests
pass (3 existing benchmark ignores). The same attested instrumented accessory ELF
passes the production `serve` path against two real v4l2loopback 0.15.3 devices in a
networkless Linux 7.0.0-34 guest. Exact black-frame bytes, 320x240 YU12 progressive
format, stride 320, size 115200 and rational 30 FPS match an independent C UAPI query.
Actual native join precedes fresh admission of both original locks. The module
unloads and the VM exits. Combined current-source native counters pass all 99
camera/control production functions individually at least 80%, with none unmeasured.
This is isolated driver/pipeline acceptance, not physical tablet/browser acceptance.

Offline host+GUI debug builds, scoped Clippy and changed-source rustfmt pass.
Pinned parser audit: 137 authored Rust/Python functions, maximum 8; native SonarQube
measurement remains unavailable under T1019. The broad formatter still reports
existing excluded EVDI drift (T1128); it was preserved. Legacy ADB ownership T1090
and joined parent acceptance T1089 remain unresolved. No install, host service or
module change, live tablet/camera action, network transfer or push occurred.
Raw manifests, attestations and profiles remain locally under
`/backups/disk2/blent-work/2026-10-19-open-items/t1137`; `export.json.gz` binds the
accepted native objects, source snapshot, guest evidence and counters.
