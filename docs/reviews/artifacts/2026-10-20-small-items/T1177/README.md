# T1177: reject exhausted viewer admission

VideoSender subscriptions now return Option<VideoReceiver>. A checked atomic
reservation precedes subscription, registry membership and capability retention.
Exhaustion returns None; the production stream workflow returns an explicit error
before requesting encoder work or exposing a streaming reader. Current callers
are updated together. There is one admission implementation and no compatibility
wrapper or rejected receiver with invented ownership.

Five permanent regressions fail before the fix and pass afterward: maximum
reservation, terminal empty membership, concurrent final admission, capability/
continuity observation and the production socket path. An independent u128 model
checks34 seeds (0–16 and MAX−16–MAX),20 transitions each. Rejected capabilities
retain no extra Arc, no receiver and no registry entry. Stale notices cannot
restore continuous membership; original Drop still removes original membership.
All8 concurrent workers are joined before original receiver handles are dropped.

Current-target verification passes65 library tests,87 binary tests and4 capture
contract integration tests (156 total). The existing T784 opt-in TCP timing
benchmark remains ignored; no regression was skipped or weakened. Fresh native
LLVM counters report100% executable-line coverage for each of the4 changed
production methods. Commands, source hashes, LCOV and red/green logs are retained.
Authored-source estimates remain <=7; native SonarQube measurement is still T1019.

A broader default cargo test also attempts examples and encountered the existing
camera-feedback-probe decoder/report compile drift. Its log is retained as
failed-default-build.log.gz, which is explicitly a failed broad build, not a passing result.
Separate T1189 records the current-caller/native-retirement repair. The passing
current-target and coverage logs name their actual scopes. No all-feature or
full-project pass is claimed.

Registry remains the sole epoch owner; bounded broadcast/storage budgets remain
unchanged. Each successful receiver owns its original membership and actual
subscription until Drop. Denial owns neither and retains no cleanup obligation.
The original registry mutex still settles membership and subscription together;
read observations do not grant cleanup authority. The terminal epoch never
revokes original receivers or substitutes for their actual retirement. Eventual
cleanup assumes original owners release their handles; these hosted bounded
tests do not prove every native interleaving or unrestricted leak freedom.
No physical device, running service, Windows/EVDI backend or network was changed.
