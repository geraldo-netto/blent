# T1346 Linux ticket settlement

Monitor retirement closes the original identity-bound display ticket before
awaiting work. RouteOwner::settle retains its original Routes view until native
cleanup completion; only then can the retirement task return the ticket for
reuse. A failed attempt is logged once and retried. Other retirement tasks keep
progressing independently. Cancelling the monitor's coroutine drops its view;
T1344's bounded original worker still owns endpoint leases and cleanup debt.

Permanent regressions cover offline cleanup, failed original removals followed by
their delayed effects, preservation of unrelated routes, replacement only after
settlement, and caller loss before reconnect. Native lease exclusion lasts until
the original worker's exit/destruction. Liveness requires original commands,
delegated effects and device cleanup eventually settle; deadlines cannot prove it.

Native credential/forwarding/launch fixtures were updated to model actual creation
and removal effects and retain their original cleanup. In particular, cancelling
an already-admitted ADB creator cannot guarantee no late server effect; the test
now requires both original mappings to be removed before replacement. Successful
fixture removals now update their snapshots, rather than falsely acknowledging
without removing anything. Existing ordering, launch-count and foreign-route
assertions remain enforced.

All 77 monitor tests passed, including the independent native-capture retirement
and sibling-progress controls. Both changed production methods have 100% native
executable-line coverage and complexity <=9. No Windows native claim is added.
