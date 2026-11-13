# Android USB/ADB transition review — HEAD 31912bc7

One confirmed actionable finding: **T1431**, Disconnect USB is disabled during a
remembered reconnect delay. The owner recorded it immediately and authorized its
fix; the separate T1431 evidence records implementation/verification.

`ConnectionControls.kt:30` enabled Disconnect only for an active connection or
pending configuration read. `RememberedConnection.follow` retains a retry timer
while both values are false, including pre-authentication failures in an already
proven retry chain. Its wait grows from 2 to 30 seconds. The retained original
`ConfiguredConnection.disconnect` path correctly cancels eligibility, but the UI
cannot invoke it during that interval. The existing permanent
`t1393DisconnectDuringBackoffAndInterveningManualAttemptWin` already establishes
the underlying cutoff behavior.

An external hosted Compose probe (`RetryDisconnectProbe.kt`, loaded with
`probe.init.gradle`) confirmed **2/2 executions**, API 27 and 34: an original
remembered/authenticated worker ends; a 2000ms timer remains; active/pending are
false; the actual Disconnect button is disabled; advancing the timer starts a
replacement. Directly invoking disconnect then suppresses the next 6000ms retry.
This is a low-severity, small-effort user-control gap, not a native-retirement or
credential-authentication bypass. Required permanent regression: click the actual
control during initial and capped/pre-auth retry waits and prove no replacement
starts, retaining untouched/fully disconnected controls.

## Review scope and clean traces

The review examined **38 selected production source files**, listed in
`source-scope.json`; inspection depth varied with the caller trace. These are not
38 implementations of a feature or a whole-project audit. Relevant manifest and
architecture/earlier-fix evidence were also checked. Eight transition areas were
traced:

1. Original USB cutoff precedes StateFlow publication; Activity replay does not
   manufacture a later cutoff. T1419's reducer/prefs adapter preserves this.
2. Null display transitions carry distinct source identities, while untouched
   startup, metadata and original retirement preserve the proper identity.
3. Protected ADB delivery before/after cutoff, backgrounding and repeated resume:
   USB remains preferred while available; later admitted delivery can restore ADB.
4. Original worker attempt/ticket/lease checks reject stale and foreign events;
   closed display owners cannot resurrect an older admitted display.
5. Process-owned display admission retains original execution, Binder retirement
   and quarantine across Activity replacement; old client cleanup cannot clear
   the new client's request. Event callbacks additionally check generation/client.
6. Control metadata callbacks execute under the shared socket/input lock; their
   queued UI work also checks the original notice. The suspected old-message/new-
   socket callback race was disproved by those two boundaries.
7. ResourceLane keeps original native ownership through failed/cancelled cleanup;
   a pending display factory is inert and displaced callbacks are detached.
8. Remembered reconnection, explicit cutoff, native attachment epochs, pending
   preference reads and Activity ownership were traced. Backend cutoff remains
   sound; T1431 is the confirmed UI integration omission.

No additional confirmed defect or documentation contradiction remained after
these traces. Untested native/API scenarios were not turned into speculative
findings; existing blocked/declined lanes were not reopened.

## Executions and limits

The initial hosted cohort passed **127/127 test executions in 19 suites**, with
zero failures/errors/skips. `cohort.log` and `cohort-results.json` preserve the
result and exact suite denominators. It covered background/null/retained/revoked
ADB delivery, session routes, display replacement/native-boundary ownership,
connection presentation/cutoff/binding/owner/display adapters, attachment replay,
configuration and remembered reconnect. This is a selected existing regression
cohort, not a test-quality campaign or total compatibility/coverage claim.

The subsequent probe passed **2/2 executions in one external suite** and
confirmed the defective state; its passing result asserts the bug's presence.
`retry-disconnect-probe.xml` preserves those observations. No permanent test was
added during review; the T1431 fix adds the required regression separately.

All execution used Java 21, the configured Android SDK, offline Gradle and hosted
Robolectric. There was no physical tablet, real USB/Binder process, installation,
configuration change or service interaction. Existing fixture-owned scopes,
virtual timers, fake workers and hosted Surface/native-boundary fixtures perform
their own cleanup. Native hardware/process acceptance remains a separate limit.
The external probe's one authored method/lambdas have <=9 complexity, recorded in
`probe-complexity.json`; no probe build output is committed.
