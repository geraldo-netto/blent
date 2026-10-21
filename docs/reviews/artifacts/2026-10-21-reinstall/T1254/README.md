# T1254 power-worker startup exit observation

The owner requested reinstallation/reload of host and Android on2026-10-21.
Current optimized host/GUI binaries were built offline from07b37456, staged with
the existing bundled external tools and unchanged EVDI helper, then registered
through the normal extracted AppDir installer. Matching cached SDK audio plugin
paths are scoped to this local launcher. No system module/package changes or network
transfers occur. Old host processes have actual pidfd exit observations
before installation; the previous AppDir remains available in the private local
work directory for rollback.

The selected RugKing Pad 2 Pro, API36, serial8002RH1010011900, receives the
existing current release APK with `adb install -r`; designated signing-certificate
verification passes. All315 recorded production source files match the APK's
earlier source manifest. The installed APK's native on-device SHA256 matches
`1ff81ab7e64088fff3eca0c1c578829e9b52317d3321c8c7ca4a009c4e97e751`.
The command sequence is: stop host service, force-stop selected Android package,
install APK without clearing data, start MainActivity, start host service, open
host GUI. This sequence records a reproduction context, not a proven minimal
reproduction or causal attribution.

Android exit-info reports a new power worker, PID20161, UID10325, at
13:39:13.043 device CEST, reason13 `OTHER KILLS BY SYSTEM`, description
`Died when creating service`. Original main31445 exits at13:39:10.491 with
reason10 `USER REQUESTED`, subreason21 `FORCE STOP`, matching the explicit
reload. Replacement power worker20306 subsequently remains alive alongside
main20068 and display20208. MainActivity is resumed. Host service279188 and GUI
280914 remain active with zero restarts; the GUI has a mapped X11 window, and
the host continues receiving tablet render acknowledgements.

The power-worker startup death is not yet attributed to expected cancellation,
installer/system behavior or an implementation defect. No continuing streaming
failure, leak or behavioral regression is established. Native exit records are
actual process-death evidence; current liveness is not proof of power-policy
correctness. Review existing startup/cancellation ownership contracts and these
bounded native observations offline first. Close T1254 with supported attribution
of expected lifecycle behavior, or diagnose and fix a defect with a permanent
normal-suite regression that fails before and passes after, individual function
coverage/complexity, and exact cleanup evidence. Do not infer a need for another
physical campaign from this observation alone.

Device timestamps use Europe/Rome, CEST/+0200; device time is approximately
2seconds behind host. Individual ADB install/start commands were not separately
timestamped. Preserve that uncertainty rather than assigning the startup exit
to one particular command. The selected-app event buffer contains only the
events still available at collection; it is not a complete process trace.

Reinstallation/reload itself passes. Current user preferences are preserved;
current schema serialization drops the obsolete `profile_cache` key, as covered
by existing T1194 regressions. Update checks remain disabled, debugging remains
on, global stay-awake remains0 and screen timeout remains15000ms. No new media
consent or physical acceptance trial is requested. No production source or test
changes are made. The raw prior configuration is retained only in the private
local rollback directory and is not included in this archive.

[observations.tar.gz](observations.tar.gz) retains selected native exit/process
observations, install/start results, host reload/render logs, GUI mapping,
artifact hashes and completion checks. [index.json](index.json) records member
sizes/SHA256 hashes. These are bounded installation observations, not new
whole-project coverage or unrestricted native-safety claims. T1254 remains open
in TODO.md; successful replacement and streaming do not resolve its attribution.
