# T1340: tablet display recovery

The black-screen trace originated in the host's permanently lost X11 power
observer. T1343 restores that observer through exact retirement and replacement.
No separate Android decoder defect was confirmed.

Installed the verified host executables and a release APK signed with the
existing designated certificate. The update preserved application data and the
existing launcher/runtime configuration. A debug-key update was rejected without
changing the installed app; the subsequent designated-key release update passed.

On one RugKing Pad 2 Pro running Android 16/API 36:

- Paused the original read-only X11 observer using its opened pidfd. The daemon
  reported observation loss, acknowledged capture inactive, and closed video.
- Observed the original pidfd become readable, proving that process exited.
  A replacement observer appeared afterward; the daemon PID stayed unchanged.
- Capture was acknowledged active and render acknowledgements resumed without
  restarting the host. This was one controlled transient-loss trial.
- Displayed a temporary owned X11 window on the tablet output. The tablet's
  captured pixels matched two distinct colors and marker positions. The second
  image followed an explicit Android force-stop and relaunch. Both images were
  captured after observer replacement and installation of the signed APK.
- A genuine host DPMS-off state correctly paused video. Waking the host
  resumed it; the session was unlocked. No power-policy setting was changed.
- The two original display mappings remained identical. USB-debugging reservation
  was retained. No camera/audio mapping acceptance or broader device-support
  claim follows from this trial.

The temporary test window was destroyed and its X11 connection closed. Native
identity/timing observations, selected host events, binary hashes and the two
synthetic screenshots are in [the artifact directory](artifacts/2026-11-12-tablet-recovery/).
Permanent failing-before/passing-after lifecycle regressions and per-function
coverage are recorded in [T1343's report](2026-11-12-observer-recovery.md).
