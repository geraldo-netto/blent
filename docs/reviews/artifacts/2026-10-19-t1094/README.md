# T1094: original setup and accessory-admission budgets

ConnectionSetupOwner and AccessoryAdmissionOwner implement the independent
ConnectionDeadlineSource capability. Each projection reads original budget/state
under its existing owner lock, without advancing the clock or changing a budget.
Active setup reports its stage budget; accessory admission reports the earlier
human-permission/native-call budget. Stopped owners report their original retirement
budget until actual retirement, close failure or observed expiration. Diagnostic
idle after a terminal condition never proves native retirement or frees admission.

Long.MAX_VALUE remains an explicit value. Existing implementations compare their
inactive sentinel against the clock, and the same number can be a valid maximum
stage/permission deadline. The projection preserves that effective limit rather
than silently mapping it to idle. No new phase flag, duplicated mutable deadline,
state representation or native behavior was introduced. Actor waits remain
unchanged; asynchronous tightened-budget/completion notifications and safe lost-hint
fallback are explicit T1092 integration work. T1095/T918 retain local aggregation
and matched performance acceptance. No polling or CPU reduction is claimed here.

Eleven permanent normal-suite cases exercise discovery, approval, readiness,
authentication, native start/request/callback limits, repeated read-only observations,
actual early cleanup, blocked original discovery/close through expiration, close
failure, late permission callbacks and genuine MAX setup/permission budgets. All
11 fail against unimplemented unknown projections, then pass after implementation.
A deliberately faulty MAX-as-idle control fails four permanent cases. Its two
original files are restored with fresh mtimes and verified before clean collection.
Raw red/green/control evidence is retained. Existing backend fixtures are reused;
one nested test-only backend becomes internal for shared use, without duplication.

The authoritative native owners, their bounded callback queues and all original
stop/quiescence/retirement predicates remain unchanged. New methods synchronously
borrow owner state and return immutable time/known values, retaining no provider,
mutable view, handle, callback or resource authority. No new native allocation,
admission, handoff, release/reuse or failure cleanup path is added. A close error or
expired budget still leaves blocked originals unretired. Conditional cleanup still
requires the exact original native workers/backend to finish; no diagnostic snapshot
or managed reference proves unrestricted native safety/leak freedom.

Tests use existing bounded fake identity/store/permission backends on actual owned
JVM threads. Their cleanup sponsors release every original gate and await existing
actual retirement predicates; simulated close failures are explicitly settled by
the fake backend before final fixture cleanup. These tests do not open USB/media
hardware or perform actual pairing/TLS. They cover the stated finite stage/fault
histories, not every native interleaving or physical Android lifecycle.

Fresh clean offline JaCoCo collection passes 71 selected new and related owner,
boundary, invalid-input and portable deadline cases with no failures/errors/skips.
All four changed production constructors/functions measure 100% native line coverage
individually. The two synchronized inline bodies are measured within their enclosing
getters by the current strict checker. Source/class hashes, raw XML/exec and method
results are retained. All 12 architecture checks pass. Pinned cached Kotlin PSI
1.9.20 audits 294 functions/callbacks across changed production/tests, maximum
repository Sonar-style estimate 7; native SonarQube remains unavailable under T1019.
No broader suite repeat or physical acceptance is claimed.

No network transfer, download, installation, restart, physical handling or push.
