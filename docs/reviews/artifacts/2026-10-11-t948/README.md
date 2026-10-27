# T948 original USB camera capture authority

Original USB camera authority retains every capture claim and sends cleanup to the original worker after UI replacement. A bounded retry relay cannot manufacture completion from socket EOF or a deadline.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The boundary projects validated private camera profiles and credentials into
portable capture targets without UI, OS or wire types in the capture contract.
Each original connection attempt owns one camera session, exact route target and
all possible lens capture claims. A closed target cannot admit another claim.

Consumer completion requires both original published CameraClosed and retirement
or no-start of every native capture claim, in either order. Logical stop, execution
end, timeout and TCP EOF supply no proof. Late closure/completion goes directly to
the original worker even after the UI has selected a replacement. Equal counters
in another private process cannot redirect proof or restore old authority. Fresh
permission/choice commands require exact current presentation and prompt identity.

The original-worker terminal relay retains one command through rejection or an
Exception, retries with exponential 20–1000 ms backoff, and holds no domain gate
across the sender. Actual original connection retirement cancels and joins the
relay; a blocked sender cannot disappear. Accepted completion ends control
admission and releases the session's retained owner. A single unresolved owner
and scalar watermark replace an unbounded historical route map. Repeated DTOs
preserve exact target identity only when port, profile and credential bytes match.
Conflicting, foreign, closed and obsolete routes cannot grant native access.

Permanent regressions were proven red before correcting the prototype's locked
sender, stop-after-retirement, malformed lease construction and stop-after-terminal
completion. Baseline binding with contract-only target/control stubs fails all four
new controller regressions; final binding passes. Tests also retain multi-lens
claims, same-counter replacement, both closure orders, rejected/throwing/blocked
senders, independent display, stale prompts and a 128-case seeded claim-order
property. Existing private Binder/actor/loopback fixture now also reports to the
actual ConnectionBinding; the joined test covers real TCP and private IPC at
simulated API 27/34. TCP EOF does not retire either retained capture claim, camera
stop preserves authenticated connection and final actual cleanup is joined.
Robolectric ServiceController construction remains on its test main thread;
production factory stays lazy. This corrects a fixture setup timeout without
weakening its routing or retirement assertions.

All 973 offline connection/camera/worker/resource/audio/display/status tests pass,
with no skips. All 45 scoped production methods meet individual native JaCoCo
80% executable-line coverage, all 7 authored constructors pass, and all 107
reviewed production/test functions meet cyclomatic complexity at most nine.
Inventory, counters, constructor counters, source hashes and red/green logs are
retained. Coverage is scoped, not project-wide.

USB capability mask remains display-only. Activity/permission observation is
T944; actual camera composition and capability enablement is T932. Pairing and
accessory authority in the integration fixture are synthetic; camera hardware,
real API 27/28 TLS/Keystore and debugging-off acceptance remain T858/T863. No
resource downloads, install/reload, live host/tablet changes or external driver
modifications.

</details>
