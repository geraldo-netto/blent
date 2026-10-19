# T1158 — explicit USB callback registration

The normal offline `lintDebug` task failed on the pre-33 permission receiver
registration. That branch already supplied the application's signature-level
callback permission. Registration now uses the installed AndroidX Core 1.12.0
compatibility API with explicit NOT_EXPORTED policy and the same permission,
receiver, filter and null scheduler. No dependency changes are required.

Cached dependency bytecode confirms the actual boundary: API33+ passes the
NOT_EXPORTED flag to the platform; API26–32 preserves the supplied permission
and passes only supported platform flags. Existing private callback isolation
and PendingIntent/selection validation remain. The old fixture overloads delegate
through explicit compatibility policy on their base context, retaining exactly
one before/after hook and the original receiver identity. The legacy detach
fixture remains a protected system-broadcast registration. Both fault fixtures
still exercise late registration, post-side-effect error and actual unregister.

This is an analyzer-visible API boundary repair. The original normal lint task
is the retained failing quality gate; no callback exposure was reproduced and
no artificial behavioral failure is claimed. Existing permanent SDK27/34 suites
cover signature permission, immutable pending intents, invalid/stale/replaced
callbacks, native grant rechecks, queue rejection, concurrent close and native
registration/cancellation/unregister failures.

The same permission owner retains receiver and PendingIntent resources through
original native return and actual cleanup. The compatibility call is synchronous
and introduces no worker, callback owner or asynchronous handoff. Failed/late
registration remains owned under the existing operation/retirement counters;
logical close and deadlines remain insufficient release evidence.

All 63 selected tests pass without failures, errors or skips. JaCoCo
0.8.15 passes all 24 scoped production functions/native variants individually at
least 80%. APK assembly succeeds; lint has zero errors and nine unrelated
warnings, tracked under T1159–T1166. Native counters and source/class/dependency
hashes accompany this record. Cached Kotlin 1.9.20 PSI reports 183 production/test entries, maximum 7;
native SonarQube remains unavailable (T1019). Hosted Android checks do not qualify
physical USB/provider behavior. No live app/device, installation, download,
network transfer or push occurred. Raw local evidence:
`/backups/disk2/blent-work/2026-10-20-open-items/t1158`.
