# T892 — portable pairing presentation and lifecycle

`ConnectionPresentation` owns immutable observable UI state without native, UI,
storage or media dependencies. Each attempt uses a new object token; each
presented peer is an exact-reference choice. Equal tickets from a replacement
process cannot revive old approve/revoke actions. Denial and revoke immediately
invalidate actions; only durable trust events change the saved-trust display.
Late identity/readiness is suppressed after stop, while relevant trust outcomes
remain visible until actual retirement. No wrapping generation is introduced.

`ConnectionBinding` injects the independent worker contract and reuses ResourceLane.
One retiring owner and one latest pending request bound replacement work; stop
never authorizes replacement. Event/reducer callbacks do not take the public
control lock, avoiding inversion with lane callbacks. Factory construction must
remain lazy, as in T854. Scope ownership lasts for the application process;
Activity integration is T893. No media permission is granted here.

Permanent coverage includes 100 queued replacement requests, exact-choice and
256 seeded foreign-ticket rejection checks, duplicate actions, native close stall,
failed retirement, failed factory/control, explicit denial and durable revoke.
The draft quarantined-replacement regression failed before its fix: failed
cleanup left a pending UI attempt marked active. It now remains inactive with
its owner retained and diagnostic visible.

Offline suite: 540 connection/lane tests passed, none skipped. All 26 inventoried
functions and 48 raw executable JVM methods in these two production files meet
80% individual line coverage; all 23 production complexity entries are at most 9.
Production and permanent test source complexity was also checked. Native/paired
acceptance remains T855; disconnected saved-computer maintenance remains T897.
