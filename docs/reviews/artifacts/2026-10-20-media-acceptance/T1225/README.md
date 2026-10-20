# T1225: tolerate transient AOA reopen I/O

Authorized T858/T860/T859 setup used the production host GUI and current signed
release APK on RugKing Pad 2 Pro (Android API36), physical port 3-2.4.4. USB
debugging was disabled before connection. Native output reported:

```
USB re-enumeration open error -1
USB probe failed: cannot open re-enumerating target
```

The tablet nevertheless enumerated as 18d1:2d00. Explicit reconnect opened the
same selected physical tablet. The production helper rejected an I/O error that
can occur during the mode transition; the old T846 test also classified this
error as permanently fatal after switching. Initial-open I/O remains fatal.

The permanent T1225 regression failed before the production change, for both
immediate and delayed disappearance. `red.log` retains that result. The bounded
post-switch loop now permits LIBUSB_ERROR_IO alongside ACCESS and NO_DEVICE.
Other error classes still fail immediately. Persistent I/O still fails after
100 reopen attempts; initial failures still make one attempt with no sleeps.
The original strict-initial assertions remain; the nontransient representative
is INVALID_PARAM instead of the now-confirmed transient I/O class. New bounded
error-class controls exercise all other statuses in -12..1.

All18 normal accessory tests pass with AddressSanitizer/UndefinedBehaviorSanitizer.
The changed production function has7/7 native executable lines covered (100%).
Native notes, counters, compiler invocations and raw gcov JSON are retained in
`native-gcov.tar.gz`; the exact source hash and function range are in
`manifest.json`. GCC/gcov13.3.0; local source complexity estimate5, with each
new test at most3. Native SonarQube remains unavailable; estimates are not a
native Sonar report.

The rebuilt helper was atomically staged beside the current source-build GUI.
It opened the existing 18d1:2d00 target and reported interface0 with no ADB
interface. This confirms actual target acquisition after the failed switch;
it does not establish a fresh post-fix cable/mode-switch cycle, pairing success
or media delivery. Those joined checks remain T858/T860/T859. Android accessory
permission/Connect USB interaction is still pending from the owner. No media
content was recorded, system audio defaults changed, unrelated device selected,
network transfer made or display-service restart performed.

`check_reopen` classifies a borrowed integer and owns no allocations. The
existing helper retains its original libusb context, exact physical-port lock,
selection check and100-attempt reopen limit. Failed opens create no new usable
handle. Successful handles retain the original claim/reader ownership; native
release/reuse still requires actual supervised process retirement. No new
cleanup path, retry after initial validation failure, or logical completion
substitute was introduced. Sanitizers and bounded fixtures do not prove
unrestricted native USB/kernel safety.
