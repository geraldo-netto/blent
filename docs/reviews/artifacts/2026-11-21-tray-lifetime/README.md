# T1475: tray application lifetime, 2026-11-21

Native tray activation started GUI 633115 in daemon 546310's blent.service group.
It would survive daemon exit and prevent service restart under the required native
retirement policy. Settings and release-page launchers now use the same independent
Linux process adapter as the shared USB broker. Stable AppImage selection, sibling
resolution, argv quoting and single-instance activation are retained. The adapter
selects a user scope before exec when a local manager exists, preserves inherited
environment and never falls back after a selected manager fails.

The permanent placement regression failed before this change. All 7 tray tests and
249 broker tests passed afterward. The private-bus tray fixture now supplies its
own runtime directory, keeping its deliberately isolated bus independent of the
real desktop manager; the new placement regression exercises the real manager.
All five changed production functions have 100% individual executable-line coverage;
all 54 measured production/test functions in the touched files stay <=9 with the
project Rust syn 2.0.119 checker. rustc 1.100.0 and cargo-llvm-cov 0.8.7 supplied
counters. Existing tests were retained; no native retirement policy was weakened.

Installed acceptance: first tray activation opened GUI 973533 outside blent.service.
Repeated activation reused its process and one visible Blent window (X11 PID/name
intersection; an initial OR search also found the editor's Blent project window).
Daemon 973398 stopped normally in 0.309 seconds and restarted as 973942 while the
original GUI, USB broker 973459 and shared ADB server 3379 stayed alive. Their
original pidfds were pinned across the restart and remained non-ready. Tablet
render acknowledgements resumed; the dashboard showed the connected tablet.
Camera/audio were left stopped. Screenshots remain local and are not published.

Ownership: the adapter constructs one owned Command from borrowed program/path
values and the process environment, with no asynchronous state or native allocation.
Callers retain original spawn/reaper admission. The GUI owns its single-instance
lease and media sessions; broker owns its independent socket/native sessions.
Daemon completion does not destroy either owner. Success, exec failure and losing
GUI/broker contenders keep the existing exact reaper paths. The unchanged broker
suite covers delayed native callbacks, caller loss and bounded starter admission.
This validates the reported lifecycle paths; it does not establish universal leak
freedom or subjective camera image quality (T1452).

Final installation identifies source commit 2c702d0c. Running daemon, GUI and
broker executable hashes match the installed optimized binaries. Both Android
package hashes match their previously verified builds; this Linux-only fix does
not change the Android wire contract. The local AppRun now persists the SDK
plugin directories already recorded in local-runtime.json, so tray launches
inherit the same local runtime selection as manual launches. The GUI was reopened
once for that environment update; display streaming stayed running. The final
installation record pins its new PID, one-instance count and launcher hash.
