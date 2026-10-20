# T897 — saved-computer management milestone

Android now lists saved computers and conditionally forgets an explicitly
confirmed fingerprint while disconnected. Complete list/empty/corrupt states,
canonical peer/snapshot admission, original store ownership and private-process
maintenance feed one application coordinator and the production settings UI.
Accepted work survives Activity/view loss; stale confirmations and clients lose
authority, and replacement waits for original cleanup. Browsing neither creates
identity material nor grants connection or media consent.

[T1152 joined acceptance](../2026-10-20-t1152/README.md) closes this milestone's
offline workflow: 203/203 combined production functions pass individual native
JVM line thresholds, all 1,403 selected JUnit cases pass, and lint/APK checks pass.
Historical component/native-lock evidence remains with T1150/T1151 and their
boundary fixes. [User instructions](../../../saved-computers.md) follow the
verified implementation. This does not close physical display, pairing, camera,
audio or older-provider acceptance (T855/T858/T846/T863), native Sonar measurement
(T1019), or the owner-blocked ADB ownership decision (T1090).
