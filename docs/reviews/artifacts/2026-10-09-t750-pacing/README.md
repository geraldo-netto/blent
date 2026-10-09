# T750 asynchronous request pacing

Partial mitigation; T750's broader stock painter-state acceptance remains open.
The new permanent `T750-async` regression failed before the change (red.log) and
passes after it (green.log). It exhaustively covers FPS 1–240 with zero/one-ms,
just-before/exact/after-deadline and 250-ms readiness; duplicate callbacks still
acknowledge GRAB and do not duplicate wait accounting. The existing invalid-FPS,
watchdog/lost-event, dirty-history and leased-buffer regressions remain enabled.
GCC/Clang ASan/UBSan pass. The full normal native suite passes, and all 207
production functions meet ≥80% individual executable-line coverage. Changed
functions remain at cyclomatic complexity ≤9.

The callback previously zeroed `last_request_ms`, bypassing the configured
request interval. It now preserves that deadline after immediate GRAB acknowledgement.
An overdue request is still due immediately. The obsolete `pipeline` field and
`BLENT_NO_PIPELINE` switch are removed: all paths use the same request-rate bound.
At 60 FPS the integer period remains 17 ms; this is a maximum request rate,
not fractional scheduling or a guarantee of exactly 60 presentations/s.

## Native comparison

Reused cached QEMU/KVM, kernel 7.0.0-34-generic, stock EVDI 1.15.1/libevdi 1.15.0,
Weston 13 GL renderer and virtio primary GPU. Only the EVDI 640×480@60 output
is enabled. Production helper FIFO is drained, configured at 30 FPS. Stock
`weston-simple-egl -f -d 40000` adds a 40-ms swap delay to exercise asynchronous
ready events. No network device, downloads, host installation or live-session
changes. Each variant runs in a fresh guest with its own firmware variable file.

A test-only copy of the owned helper adds bounded trace calls around request,
ready, GRAB begin/end, damage count and publication (maximum 6,000 events; neither
run hits the cap). Both variants use identical instrumentation; production has
no new logging. Original logs, including client Wayland protocol traffic, are
retained compressed. Fixture snapshots use local cached-asset paths; rebuild
from pristine source copies before applying `instrument.py.txt`.

| Observation | Before | After |
|---|---:|---:|
| Requests | 511 | 371 |
| Ready callbacks | 486 | 5 |
| Request intervals below required 34 ms | 372 | 0 |
| Minimum request interval, ms clock | 1 | 34 |
| Median request interval, ms clock | 30 | 34 |
| Ready events missing GRAB before next request | 0 | 0 |
| Nonempty grabs/s, steady five-second windows | 47.3 / 35.7 | 29.3 / 29.3 |

Request intervals use the actual scheduler's millisecond clock. Instrumentation,
VM scheduling and concurrent test load affect absolute costs; this comparison
checks deadline enforcement, not microsecond performance. Client five-second
counts vary from 18.8–23.6 FPS before and 19.4–20.6 after: no client-FPS gain is
claimed. Fewer requests/grabs are the intended cap, not evidence of frame loss.

## Remaining acceptance

Trace order verifies owned-code GRAB acknowledgement and request pacing. Wayland
commits/frame callbacks are not physical presentation feedback or kernel atomic
commit identities. The libevdi callback provides buffer ID 0 without a per-frame
identity; these traces cannot establish snapshot atomicity or exclude upstream
painter races, loss, damage reordering or tearing. A deterministic frame-pattern
and presentation-feedback fixture, or stock upstream evidence excluding the
path, remains necessary. No driver patch is authorized. T750 stays open with
those exact requirements instead of treating a rate limit as a driver-race fix.
Existing T748 watchdog warnings after inactivity remain visible and separate.
