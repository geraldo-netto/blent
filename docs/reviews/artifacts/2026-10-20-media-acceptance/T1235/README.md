# T1235: foreground media screen demand

A camera-only session previously supplied no screen demand when the display
source was unavailable. The actual tablet's15s screen timeout ended foreground
capture. Permanent actual-Activity regressions reproduce the missing Android
Window flag on API27/34 before the fix; both pass afterward.

ScreenWakeDemand is an independent read-only contract. DisplayPowerDemand now
implements that narrow contract. MediaScreenWakeSource borrows the three
application lifecycle views, with original-snapshot identity checks. MainActivity
composes display/media demand into the existing single WindowPowerBinding owner.
No duplicate flag writer, compatibility overload, new native wake lock, media
Start, consent grant or Android system-setting write is introduced.

All12 camera/microphone/speaker states are checked, plus original snapshot
withdrawal/equal replacement, independent display/media demand, faulty readers,
Stop and observer cutoff. Existing display pause, stale owner, reentrancy,
blocked native retirement and real Activity lifecycle regressions remain.
The final ordinary cohort passes82 cases. JaCoCo0.8.15 measures every changed
production function individually at100%, including the new constructors and
separately compiled callbacks; raw counters/classes and source hashes are saved.
Kotlin PSI1.9.20 independently estimates every function in the changed scope at
<=9. This is source counting, not unavailable native SonarQube measurement.

The same-identity release APK is installed. With desktop sharing stopped, an
actual Front camera worker(PID5395) delivered12 640x360 frames with11 distinct
hashes. The actual MainActivity window has KEEP_SCREEN_ON. The temporary global
USB-awake override was restored exactly to0 before the idle observation;
Android retained its15000ms screen timeout. More than35s later the same worker
still delivered12 frames/11 distinct hashes and power remained Awake. The owner
then pressed Android Stop: the original camera worker disappeared and the actual
Window no longer had KEEP_SCREEN_ON. Desktop sharing was restarted. No image or
audio content is retained. This camera trial does not close debugging-off or
complete audio acceptance, which remain T858/T860/T859.

The Activity owns the window observer; Android retains the borrowed Window.
Application camera/audio owners remain authoritative for lifecycle views and
native obligations. The new source owns only bounded values/references to three
StateFlows, with cold conflated observation inside the original Window scope.
Old snapshots cannot revive after withdrawal/equal replacement. Stop cuts off
that scope before clearing the flag; stale callbacks retain the original owner
check. Failures terminate only the original observer. Activity destruction still
clears its own Window, and a replacement Activity owns a different Window.

Screen intent is never native acquisition, quiescence, settlement or reuse proof.
The actual-Activity test deliberately clears camera demand while its original
private worker still lacks retirement proof. Existing media owners retain their
resource fences and cleanup sponsors across partial failure, cancellation,
permission loss, source loss and exhaustion. A window write failure leaves any
unknown OS obligation with its original framework Window/lifecycle owner. The
bounded truth table, hosted native counters and physical camera observation
complement one another; they do not prove all Android devices or unrestricted
native leak freedom. Home/manual lock preserve foreground/background consent.
