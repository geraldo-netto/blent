# T1081: shared auxiliary activity source

One per-process Linux Registry composes the original activity_follow::Following
and logind-only PowerFactory. Camera and both audio workflows may share one
read-only ObservationLease contract. Native integration remains T1082/T1083;
this prerequisite does not claim physical media or suspend/resume acceptance.

A dedicated current-thread Tokio runtime runs on its own original OS thread,
independent of the first consumer runtime. Registry admits one root before
creating that runtime/thread; concurrent opens serialize and upgrade one Weak
root. The root owns the original worker handle/source, without a second mutable
publication or a root/thread retaining cycle. Failed/stopped originals reject
new clients. Last lease release requests stop through a private one-way watch
channel and transfers the original thread/source to retained retirement. This
release cannot synchronously stop Following: a caller may hold its read guard.
Derived publication receivers grant no native authority. Guards must quiesce
before publication writes/destruction can complete.

The retained observer first joins the exact original thread, including thread
local destruction, then retains source stop/destruction in an off-reaper Cleanup.
Even after worker unwind, source.retired() must account for the independent
native SourceLifetime obligations before root capacity is reused. Neither last
reference release, logical source stop nor worker join alone proves native
retirement. Existing configured activity/native and OwnedTask budgets remain
independent sponsors. An uncertain original remains admitted and funded. The
single root is an ownership topology; Settings.interval also controls retained
native retirement polling. There is no elapsed-time kill or automatic retry of
failed global initialization. Blocking initialization itself runs inside an
admitted OwnedTask constructor. Cancellation retains that exact constructor;
its rejected late root disposes its own worker through the same retirement path.

Storage is bounded to one weak current root, one original worker/root admission,
one bounded startup result and one constructor result per admitted opening.
Each caller owns its lease; all media authority stays elsewhere. Partial startup
errors transfer the real thread even before a source is exposed. Underlying
source lifetime independently retains native tasks that outlive runtime teardown.
Cleanup is conditional on actual native completion, released borrowed guards,
fair scheduling/polling and eventual cleanup evidence. No reference-count or
sanitizer result substitutes for those conditions or proves unrestricted safety.

Fourteen permanent hosted cases cover three separate short-lived client runtimes,
all six three-consumer release orders, original power events, late creation,
cancelled blocking construction, exhaustion, invalid settings, cached errors,
failed/panicking backends, owner loss before startup receipt, held native work,
borrowed-view responsiveness, exact original thread join and native retirement
remaining after a thread has joined. Test gates are caller sponsored: sender
loss also releases held work, avoiding test resource cycles. Private D-Bus tests
use an isolated fake bus and never change host or tablet power. Unsupported
logind capabilities remain explicit. Fixture state bounds are eight original
identities, four pending gates/events; configured native capacity is respected.
Deliberate blocking-release and missing-thread-fence controls fail their permanent
cases, with original gates/joins cleaned before assertion. Restoring a control
with an old filesystem timestamp initially reused its bad Cargo executable;
touching the restored file forces recompilation. Final counters use an exclusively
owned fresh target and source/object attestations, so this probe timestamp artifact
cannot contaminate the final result.

Final measurement and tool versions are recorded alongside this document.
Source audit is an estimate using repository counting functions and cached
pinned tree-sitter-rust 0.24.2/tree-sitter 0.25.2, not native SonarQube (T1019).
All 12 architecture checks pass. No device handling, installations, network
transfers or support claims were added. Truthful retained-intent status and
original transport-target binding remain explicit prerequisites T1085/T1086.

Final offline LLVM collection: 14 passing cases; all 17 maintained production
functions measure 100% executable-line coverage individually. Source audit: 52
functions including tests/fixtures, maximum estimate 6. The earlier 71.43%
settle_source result is retained as missing-coverage evidence before adding the
permanent independent-native-retirement test; no exclusions or weaker thresholds.
