# T894 — production host USB pairing controls

The USB connection tab lazily composes the portable Client port with the Linux
presentation Factory. Widgets render complete snapshots, matching Computer and
Tablet fingerprints, explicit device Connect, approve/deny/remember, Disconnect,
and saved-tablet revocation. Remember choices survive unchanged refreshes but
expire with the exact pending ticket. No queued command is treated as approval;
one mutation remains pending until its matching T903 outcome arrives. Rejections
remain visible across polling. Cancellation or transport failure leaves the
command explicitly unconfirmed. Native cleanup remains visible after a terminal
status and retry cannot replace its owning Factory or bypass real retirement.

The independent native composition adapter resolves absolute runtime/state roots
without scanning disks or querying devices on the GUI thread. Local acceptance
expects blent-usb-broker and blent-usb-io beside the GUI. State uses
XDG_STATE_HOME/blent/connection (HOME/.local/state fallback); runtime uses
XDG_RUNTIME_DIR/blent-usb. Missing binaries produce an asynchronous failure.
Windows ADB-free USB remains visibly unsupported (T902); packaging remains T861.
Stopping discovery or closing the window leaves independent tablet connections
running, as the UI states; Disconnect explicitly stops the selected connection.

Permanent widget tests exercise exact commands, labels, bounded invalid targets,
queue rejection, stale tickets, refresh preservation, narrow layouts, terminal
cleanup and source reuse. Two draft regressions failed before their fixes:
transport diagnostics hid command uncertainty, and a terminal status hid pending
native cleanup. Both permanent tests remain. The native factory runs in an
isolated child, proving construction opens no devices/storage; missing binaries
fail with actual client retirement. App routing preserves every settings tab.

98 GUI unit tests and one actual GUI/Xvfb/accessibility startup test pass, none
ignored. All 20 new production functions and four changed App wiring functions
meet 80% individual executable-line coverage (App::new 94.23%; others 100%). All
51 scoped production/test functions have complexity at most 9. Stale mappings
from the reused target reproduced T880; acceptance exports ONLY the freshly built
blent_gui-4d820be0a8b4421a test executable and blent-gui executable with the native
merged profile. Source hashes, original native counters, manifest and logs remain.
No threshold or test was weakened. Native Windows validation remains unavailable.

This implements pairing controls, not USB display/audio/camera. Joined production
Android/host physical acceptance remains T855; media integration T857–T860. No live
USB mode change, installation or download occurred during this item.
