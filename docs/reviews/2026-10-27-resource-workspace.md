# Native tablet-resource workspace (T1273)

The native workspace implements independent display/input, camera, microphone and speaker controls bound to exact tablet identities. Tests preserve queued starts, cleanup ownership, drafts and sibling resources across navigation. Linux startup and per-function checks passed; screenshots use simulated inventory, later reports refine navigation/layout, and native Windows acceptance remains unavailable.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

Implemented the accepted T1268 resource workspace and owner-selected per-tablet
display controls, using T1272 contracts and the T1274/T1275 native backend.

The home screen presents selected-tablet display/input, camera, microphone and
speakers. Wide windows use a tablet sidebar and two-column resource cards;
compact windows use a selector and scrollable single column. Connection presence,
unknown external activity, assignment versus video delivery, waiting, retirement
and backend limits stay explicit. Native display-owner metadata remains visible
when independent ADB discovery fails. Media-only catalog entries are never labeled
as display assignments.

Each resource action uses the selected full transport identity. Tablet widgets and
resource cards also bind their input IDs to that full identity: pointer gestures
cannot cross selection changes. Friendly names and shortened fingerprints are
presentation only. Camera, microphone and speakers retain their existing one-owner
capacity independently. An occupied resource remains visibly reserved elsewhere.

Workspace Start clones preferences with the exact selected target; it does not
rewrite saved drafts. Configure opens existing resource editors. Save/Discard
appear only in editors, and a disruptive save explicitly names all displays.
Native media starts only through explicit controls; saving and navigation do not
start capture. Editor Start uses the editor's target fields. Original automatic
ADB selection and all existing settings remain available. Global service controls
stay in Settings, explicitly labeled as affecting all tablets.

One panel owns each media admission's immutable target and queued-start metadata.
The native controller remains authoritative for activity. A queued Start cannot
expose the earlier Stopped observation as free capacity. Stop consumes that
controller and retains its original native retirement fence before replacement
or retargeting. Changing selection, drafts, capability reports or views cannot
release it. Unknown native outcomes remain funded by the existing worker reaper.
The UI stores no second native lifecycle or OS handles. Frame projections borrow
current observations; catalog/profile data and input strings remain bounded.

The catalog admits at most 1,024 identities per frame. Current daemon assignments,
authenticated USB links, explicitly refreshed ADB observations and retained media
owners merge by identity. A saved/last-seen identity does not certify a current
connection. Discovery does not start merely by navigating between resource cards;
USB discovery and refresh remain explicit. Native IPC stays inside the platform
adapter, and the existing bounded background worker keeps polling off rendering.

Typography, contrast tokens, spacing and 32-point interaction rows improve
readability and targeting. Configuration grids budget slider widths at narrow
sizes; short editable hints retain full selection guidance on hover. Detailed
resource/action errors have bounded scroll areas. Project links move to Settings.
Native platform theme selection remains available. This is not a claim of complete
screen-reader or usability-standard conformance.

Permanent regressions cover queued retargeting, original retirement, identity/name
separation, all resource activity states, stale selection across pointer gestures,
ADB-independent display owners, media-only assignment filtering, per-resource
actions, passive drafts/navigation/refresh and named keyboard/accessibility actions.
The queued-retarget, assignment-filter and pointer regressions failed before their
fixes. The missing-owner regression failed against the original omission control.
Existing narrow-window tests reproduced the enlarged-slider/hint overflow before
the layout correction. All prior tests remain in the normal suite; custom editor
fixtures use a tall canvas while dedicated small-window checks retain real bounds.

Final offline evidence:

- 148 GUI unit tests pass; the actual workspace binary passes native Xvfb/dbus
  startup and accessibility-bridge checks. Final GUI suite was repeated after
  integrating the T1275 backend correction.
- All 102 changed Linux production functions have >=82.6087% executable-line
  coverage individually, including actual main/window creation. Rust 1.98.1,
  cargo-llvm-cov 0.8.7, LLVM 22.1.8; source hashes and per-function results are in
  [verification.json](artifacts/2026-10-27-t1273/verification.json).
- Locked syn 2.0.119 measures 448 GUI functions in the changed modules and their
  tests, maximum complexity 9. The full installed project checker measures
  28,394 functions with zero above 9. No suppression or coverage exclusion added.
- Windows tests cross-compile offline. Native Windows counters/window behavior
  remain unavailable under T1271; compilation does not establish runtime support.

Real-window previews: [desktop](artifacts/2026-10-27-t1273/workspace-desktop.png),
[compact](artifacts/2026-10-27-t1273/workspace-compact.png). These use two simulated
ADB inventory records in a private desktop session. The read-only existing service
lacks the new endpoint, demonstrating contextual upgrade/restart guidance. No
resource Start/Stop, capture, installed-service change, installation or external network
transfer occurred. Owned GUI windows/processes exited normally and temporary
fixture directories were removed. Logs and LCOV remain in owned work directory
`2026-10-26-t1267-t1270`.

</details>
