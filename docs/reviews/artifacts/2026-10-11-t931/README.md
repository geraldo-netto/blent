# T931 portable camera invitation and target contract

Camera invitation contracts separate transport acceptance from capture permission and validate explicit USB targets. At this stage native sharing remains disabled pending composition.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Camera sessions depend on the independent `camera::connection::Invitation` port.
The existing ADB adapter accepts the portable profile rather than Linux output
device options; native device/capture/process ownership remains outside the port.
Invitation success is transport acceptance, never camera permission or capture.

Explicit `usb_peer` selection validates the exact lowercase certificate fingerprint
and rejects a simultaneous ADB serial. Automatic selection remains ADB-only;
selecting a USB display never redirects camera. Existing saved profiles default
to their original ADB selection, and absent USB fields do not change their output.
The CLI exposes the selector but native camera sharing is explicitly unsupported
until T933 composition, rejected before opening any camera output device.

Checked projection preserves all existing remote camera fields. Rotation/mirror
remain host decoder transforms; neither OS paths nor transport selectors enter
the wire schema. Credentials retain their existing secret 64-character hex format.
The permanent ambiguous/invalid target regression failed before validation and
passes unchanged after it. Bounded tests exercise every ASCII credential/peer byte,
0–128 byte extents, narrowing rejection, CLI conflicts and saved settings. Independent
session/ADB tests verify the invitation port and refusal to misroute USB through ADB.

All 279 common tests, the existing GUI camera-settings regression and 29 host
camera tests in each default/native build pass, including the original real-H.264
codec/feedback, private PTY/device-lock/mount and native cleanup fixtures. All 21
reviewed production functions meet individual 80% executable-line coverage; all
54 reviewed production/test functions have cyclomatic complexity at most nine.
Exact freshly executed common/native host mappings supply LLVM counters. The
minimal no-platform common build and Windows production/test cross-check pass.
This is contract readiness; Android camera permission/lease composition and native
USB camera acceptance remain T934/T932/T933/T858. No download, installation, reload
or physical tablet trial occurred.

</details>
