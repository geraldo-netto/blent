# T1202: probe transport and limit admission precedes native effects

The actual entry-point camera/duration preflight was extracted to a narrow
CameraProfile policy before changing behavior. Two permanent tests first fail:
a valid explicit USB target is admitted to the ADB-only probe, and an invalid
stall delay is admitted before native setup. The old proxy rejected delay only
after listeners/mapping creation; the route path ignored USB identity and could
automatically select an ADB tablet, contradicting the portable Target contract.

Policy now declares ADB support and USB unavailability through the existing
portable Transports contract, validates the profile/target and enforces independent
1–30 second / 0–1000 millisecond limits. The current run entry point calls it
before any listener, discovery or ADB operation. Remove the proxy's duplicate
late stall check; it receives only the private validated workflow's value.
No USB implementation or alternative target fallback is added.

All five normal-suite tests pass. Preserve the two red oracles plus automatic/
explicit ADB and exact limit endpoints, 66 USB identifier lengths, conflicting
selectors, invalid geometry and unsigned maximum durations/delays. Tests call the
same portable gate as production and perform no ADB/device action. Command:
cargo test --offline --locked -p blent --test camera_probe_policy. Formatting and
whitespace checks pass. This diagnostic benchmark is exempt from production line
coverage. Manual source decision upper bounds are <=6 for new/changed methods;
native SonarQube remains unavailable (T1019). The pre-existing full probe decoder
compile/retirement integration remains T1189; no native acceptance is claimed.

The validation function borrows an immutable portable profile and scalar limits,
returning only owned errors or unit. Its containing lifetime is the synchronous
entry-point preflight. Route selectors never become native handles here; no
allocation ownership, callback, queue or reuse protocol is invented. Caller-owned
configuration remains intact on rejection, and a denied target cannot reach the
native constructor. Current borrowed target views stay within the profile's
borrow; shared parsers retain their existing bounded selector contracts. No
physical camera/tablet, Windows/EVDI change or network transfer.
