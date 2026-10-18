# T1061: current display demand and Activity Window

Original control observations now drive native wake and actual Window demand.
Pause/source loss deny wake before queued UI delivery; authentication and current
metadata gate activation. Supported-source history survives coalescing. Fresh
legacy greetings retain explicit Unsupported fallback; background denies demand.
Pure source permits retain no UI/socket/notification subscriber. ControlSession
hints and observable foreground/settings/metadata state provide bounded notices.

Window ownership starts with foreground, not onCreate. Original epoch checks
fence stale/reentrant writes and prevent old observer installation. Canceling the
input clears original screen demand without reporting normal cancellation. Flow,
write and reporter faults remain contained. Window and sensor cleanup are both
attempted; the framework retains its actual Activity Window lifetime. Native
retirement remains independently owned and fenced by the application runtime.

Permanent pre-fix regressions fail on SDK 27/34 for paused-source wake, background
wake and unconditional onCreate flags. A draft canceled-input regression also
fails on both SDKs before withdrawal. Further cases cover held actual native
retirement, replacement/coalesced loss, optional UI failures, current metadata,
settings, legacy fallback, old observer cutoff and actual subscription settlement.
Existing T807/T1064 and T388 tests retain their behavioral assertions: private
started reflection follows its observable representation, and legacy-host expected
policy explicitly names Unsupported instead of the former Waiting placeholder.
The preliminary full run's two stale expected-state failures remain retained.

Final source-attested clean offline suite: **2,224 tests, no failures/errors/skips**.
Every **39** maintained changed production functions/callbacks has **100%** native
JaCoCo 0.8.15 line coverage. Raw execution/XML counters, complete source/vector
and compiled-class hashes, source scope/inventory, logs and fault controls are
retained. The final progress log contains two complete collections (the first
failed, the second passed); suite-summary.json describes only the final collection.
Kotlin PSI 1.9.20 source audit covers **629** production/test functions, maximum
estimate **8**; native SonarQube remains T1019. Twelve architecture/contract/
portability checks pass. Existing compiler warnings remain T1072.

Joined acceptance remains T1058/T1011. No physical Window/kernel/process or power
measurement is claimed; T914 retains that owner-approved native window. No network,
installation, push, live lock/suspend or tablet interruption occurred.
