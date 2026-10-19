# T918: measured Android actor idle fallback

Parent revision: `2d0514ab`. Production tuning changes only the default
`ConnectionWorkerSpec.timerMs` from 10 to 100 ms. The independent 1–1000 ms
configuration range, 10 ms unknown-provider/retirement fallback, earlier absolute
deadlines and configured asynchronous budget caps remain enforced.

## Measurements

The opt-in tool is `scripts/benchmarks/android-connection-idle/run.py`; its README
defines finite measurement domains, native counter boundaries and cleanup.
All runs used installed cached tools, Gradle offline and the local JDK
`21.0.12.1+1-1-24.04.4-Ubuntu`. No device, installation, restart or network use.
Results concern the production actors with **synthetic idle native owners**.

Each of two actors has one original executor thread. Each trial warms up for
one second, then measures three seconds before any latency stimuli. Order is
10/100/1000/1000/100/10 ms, followed by an unknown-source 1000 ms control. Each
actor then receives 32 hinted changes and two deliberately missed hints.

| Known fallback | Idle advances per actor | Combined actor CPU per window | Combined actor allocation | Hinted p95 | Missed hint latency |
| --- | --- | --- | --- | --- | --- |
| 10 ms | 292–297 | 24.84–83.44 ms | 523,936–525,648 bytes | 0.083–0.114 ms | 7.047–7.115 ms |
| 100 ms | 30 | 7.36–8.94 ms | 52,976 bytes | 0.053–0.165 ms | 97.073–97.256 ms |
| 1000 ms | 2 | 0.58–0.70 ms | 3,488–3,608 bytes | 0.067–0.145 ms | 997.090–997.312 ms |

The unknown-source control still advances 295 times per actor, with missed hints
around 7.1 ms. Whole-process CPU was 70–280 ms at 10 ms and 20–30 ms at 100 ms;
it includes compilation/GC and is coarse enough to report zero for the 1000 ms
windows despite positive thread CPU. The first trial shows warmup sensitivity.
These are two bounded windows per known setting, not a statistical device study.
100 ms reduces idle work while retaining a roughly 100 ms missed-hint bound;
1000 ms imposes a roughly one-second missed-hint wait. Hinted latency remains
independent of the fallback in this experiment. No Android battery or complete
media performance claim follows.

The separate allocation probe warms up one million and measures two million
iterations per case. Direct nonescaping `at(null)` allocated 4,529,448 bytes;
deliberately escaping it allocated 48,000,168 bytes. Consuming
`at(null).combine(IDLE)` allocated 96,000,168 bytes with and without explicit
escape. Scalar replacement therefore cannot be assumed across these paths.
This measures a hosted JVM call pattern, not ART; T1097 tracks immutable reuse.

Raw JSON, exact input hashes, logs and source copies are retained. `actor-inputs`
contains the initial actor tool version; the later allocation run adds a
separate entry point and reflective native-counter adapter visibility only.
Both runs precede the default change and explicitly select each tested fallback.
`analyze.py` reproduces `summary.json.gz` from the two raw result directories.

## Verification and ownership

The new normal-suite `t918MeasuredIdleDefaultPreservesDeadlineAndRetirementBounds`
fails before tuning (expected 100, original 10). The prior T1092 default assertion
follows the selected new default; all its boundary, invalid-input and budget
assertions remain. Fresh actor/deadline/codec tests and strict native JaCoCo
counters pass; the changed production constructor has 100% executable-line
coverage. `tests.json`, original XML/exec, class/source hashes and per-function
report retain the exact scope. Twelve architecture checks pass.

Two permanent Python tool regressions reproduce and fix early AGP task lookup
and unavailable management APIs on the Android compilation boot classpath.
The fixes defer task lookup and access hosted counters through public JVM
interfaces without changing application compiler settings. Red/green logs
are retained. The final combined tool run is `tool-green.log.gz`.

Each trial owns its original scope, actor, dispatcher, thread, cue and receipt.
Success/failure cleanup cancels and awaits the exact actor, retires it, closes
its dispatcher and joins its original thread before releasing the trial.
Counter objects borrow JVM-owned management beans. Allocation probes retain at
most one immutable escaped snapshot, cleared in `finally`. No native resource
ownership, admission or reuse contract changes in production. Cancellation and
timeout retain their original owner/fence regressions; a deadline is diagnostic,
not native retirement evidence.

Kotlin PSI 1.9.20 source estimates cover 108 functions/callbacks in changed Kotlin
files and benchmark sources, maximum 8. The repository Python AST visitor covers
five functions, maximum 3. The small Gradle registration closures were manually
reviewed (at most two decisions each); no supported Groovy metric is installed.
These are not native SonarQube measurements; T1019 remains blocked. Benchmarks
and nonessential development scripts retain the established coverage exemption.
