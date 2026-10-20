# T1233: distinguish USB media from desktop discovery

An authenticated USB media connection used the desktop-only waiting screen,
including USB-debugging instructions. Actual owner confusion reproduced this
while the tablet used accessory18d1:2d00 without an ADB interface.

The main screen now borrows ConnectionBinding's authenticated projection and
shows USB media connected when no desktop is streaming. Its button opens the
existing camera/audio controls directly. Calibration keeps its own status;
streaming keeps the video surface. One nullable destination owns which sheet is
visible. Opening or dismissing controls grants no consent, starts no native work
and cannot disconnect the original media owner.

Both permanent actual-Activity regressions failed before the fix on API27/34
because authenticated status was absent. They pass after the fix, including
closing/replacement identities, stale reports, original camera offer routing,
no automatic media or display admission and independent sheet dismissal.
The final ordinary UI/lifecycle cohort passes82 cases without failures/ignores.
JaCoCo0.8.15 measures all six changed/new named functions individually at100%,
plus the changed callbacks and new sheet bodies. Exact raw counters/classes,
source hashes and red/green logs are retained. Kotlin PSI1.9.20 estimates named
complexity9/9/6/4/1/1; this is independent source counting, not native SonarQube.
Native Sonar remains unavailable; no tool download.

The same-identity release APK was installed without clearing data. Native tablet
XML now shows authenticated media without the misleading desktop-search text;
its direct controls button opens the actual camera/audio sheet. The owner also
confirmed USB media connected and used the new button. Display sharing was
briefly stopped for the status check and restarted in finally. This is UI
acceptance, not completed debugging-off camera/audio acceptance.

The Activity composition borrows one immutable authentication value and existing
controls. StateFlow collection and callback capture end with composition; sheet
state is Activity-local, with no retained native allocation or OS handle. The
existing application owners retain media consent, leases, cutoff and actual
retirement. Authentication is a presentation input, never release/admission
proof. Stop, loss, replacement and Activity teardown preserve original owners;
the tests settle their scoped workers before restoring process-owner fixtures.
