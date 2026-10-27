# T558: renewed stock-library review

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

Upstream had released
[v1.15.1 on September 15, 2026](https://github.com/DisplayLink/evdi/releases/tag/v1.15.1).
The September 27 review and its retained source fingerprint incorrectly named
v1.15.0 as the latest release. This metadata error was corrected on October 9
under T754; the archived JSON retains the original observation. Blent's release
build still pins stock libevdi v1.15.0 in `scripts/build-release.sh`.

Inspection of upstream main at
[`33b7b3c`](https://github.com/DisplayLink/evdi/blob/33b7b3c9f7c77157c2b1b0c5a40cebe19b8ba110/library/evdi_lib.c)
on September 27 finds the same process-scan/sleep loop in `wait_for_master`.
The [source fingerprint](artifacts/2026-09-27-evdi-acquisition/upstream.json)
records that exact revision; this is source inspection, not a new device trial.

The [v1.15.0-to-v1.15.1 comparison](https://github.com/DisplayLink/evdi/compare/v1.15.0...v1.15.1)
does not change `library/evdi_lib.c`. Its kernel reconnect fix does not provide
a bounded library acquisition API, so the T558 conclusion below still applies.

The loop can scan process metadata 51 times and sleep 50 times. It counts sleeps,
not elapsed scan time. The existing read-only reproduction measured 11.741 seconds
wall/6.736 seconds CPU where root-owned Xorg metadata was unreadable. The API still
has no caller-supplied scan budget or supported open-from-fd entry. Attached-device
opening ultimately uses `evdi_open`, so switching public entry points is ineffective.

Ownership remains separate from the heuristic: `open_as_slave` opens the DRM node,
checks master status, attempts to drop master when necessary, then checks again
before returning a slave descriptor. Blent adds its exclusive inode lease and
connected-output recheck. Any optimization must retain all these checks and close
failed descriptors; an unobservable owner is not permission to claim a device.

A narrow candidate is a monotonic elapsed-time budget checked within process
scanning, retaining the existing wait limit and subsequent DRM checks. That would
bound repeated metadata work; it would not establish that unreadable metadata
means no master. Native permission failures, master-to-slave transitions, timeout,
shutdown and lease contention would need permanent regressions and native evidence.
A cooperative deadline also cannot promise to preempt an arbitrarily blocked
kernel filesystem operation.

The September 21 decision requires stock libevdi; `packaging/appimage/sources.py`
also requires the unmodified pinned source. On 2026-09-30 the maintainer reaffirmed
stock libevdi; the proposed local patch is not authorized. T558 remains blocked on
a stock ownership-preserving bounded acquisition API, unless the maintainer later
revises that dependency policy. Any implementation still needs ownership validation.
No dependency, DRM device,
privilege, live module or running helper was changed. The earlier timing is a
historical observation, not a newly benchmarked result.
