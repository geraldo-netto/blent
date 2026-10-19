# T1099: persisted Android USB capacity/deadline controls

Parent revision `4246faaf`. Settings exposes all eleven portable preference
fields with bounds and memory/latency/cleanup tradeoffs. Manual Connect and
accessory attach use the same saved immutable snapshot, then recheck current
connection authority. Saving affects the next connection and changes no active
worker, trust, permission or media consent. Host parity/joined acceptance remains
T898; physical restart/persistence acceptance remains T846, and T863 retains its
older native provider/accessory qualification limits.

## Verification

Fresh clean complete Android suite: **2,345 tests; zero failures, errors or
skips**. All **66 mapped changed/new production method/initializer entries have
100% executable-line coverage individually**. The authored inventory has 75
entries; inline bodies are measured within their enclosing method and separately
compiled callbacks retain their own counters. Original JaCoCo XML/exec,
class/source hashes, inventory, strict per-function results and normal-suite XML
are retained. No coverage exclusions, threshold changes or new skips.

Twelve architecture checks pass. Kotlin PSI 1.9.20 source estimates cover 314
functions/callbacks, maximum 6. Retained temporary layout prototypes add 99
estimated functions/callbacks, maximum 3. These estimates are not native
SonarQube evidence; T1019 remains blocked. Tooling ran from installed caches,
offline, without installing/restarting applications or handling a tablet.

Permanent regressions and fault cases cover:

- A backend cache changing before failed publication: two tests fail before
  the quarantine fix, then pass unchanged. Connection reads remain rejected
  until an explicit successful save resolves the unconfirmed publication.
- An already-cancelled save returning a validation result: the new test fails
  before the entry cancellation check, then passes in the final complete suite.
- Exact original blocked read/write calls: cancellation retains the API slot;
  64 competing read/save attempts per mode reject without queueing native work.
  The fixture releases its original block, joins its original coroutine, closes
  its dispatcher and joins its exact executor thread on every exit.
- All **48** finite read/write, owner-loss, competing read/save and original
  return histories: 24 distinct four-event orders per original operation. The
  oracle preserves an occupied slot until the explicit original-return event.
  The first fixture incorrectly drained that operation during rejected-probe
  cleanup; the retained history test exposed this. The corrected fixture keeps
  the same oracle/assertions and drains unexpected admitted probes only for
  failure cleanup. Every history settles its original job and virtual queue.
- Native preference adapter save/reopen API round trips, corruption/type errors,
  bounded writes, and application-context-only ownership on hosted SDKs 27/34.
- Invalid drafts, failed save/retry, default/reload behavior, bounded errors and
  oversized edits; cancellation prevents stale draft/start publication.
- Existing pairing/legacy-credential assertions, actual Activity manual/attach
  composition, recreation, active-owner isolation and explicit media consent.

The adapter round trip exercises hosted Android APIs; it is not evidence of
physical device storage persistence across a process restart or power failure.

## Dialog failure and measured correction

The unchanged 640×360 regression initially fails before its first edit: SDK27
does not become idle; SDK34 exhausts the hosted trace heap. A bounded diagnostic
records **384** measurements, at most 32 for each of twelve observed nodes.
The static form alternates width constraints 272/512 px and resulting input/body
widths 272/280 px. The stock platform intrinsic-width probe and the form's minimum
input width repeatedly request incompatible layouts.

Explicit content width, bounded by 560 dp and the available width, together with
`DialogProperties(usePlatformDefaultWidth = false)`, removes the oscillation.
The original scroll body and fixed action slots remain. All eight unchanged UI
cases pass on SDKs 27 and 34, including the original small-window test. No heap
limit, idling deadline, window qualifier or assertion was weakened. Temporary
instrumentation was removed after diagnosis; the permanent regression remains.

Two unsuccessful layout experiments are retained as **failed/incomplete probes**,
separately from the final full green suite. The bounded-dialog experiment was
contained after three confirmed layout failures and prior trace exhaustion; the
smaller-viewport experiment exhausted its test heap. Cleanup signaled only the
original owned test JVM, using a pidfd after checking its start identity and
ancestry to the Gradle process writing the exact experiment log. The latter
required SIGKILL after graceful termination did not finish. Parent commands
observed actual worker exits. These stops address confirmed resource failure,
not an elapsed execution budget. No incomplete probe is reported as passing.

## Ownership, cutoff and settlement

The application owns one preference store and the backend retains only
application context. `Mutex.tryLock(originalIdentity)` admits one original
storage API call, with no waiting queue. Immutable input is bounded before
publication. Cancellation is checked before validation/admission; an admitted
call runs in its retained noncancellable I/O context. The same identity is
unlocked only after the actual API call returns. Backend/internal framework file
and writer resources remain framework-owned; this is not a separate native
file-descriptor fence. No application native handle or mutable buffer loan is
released or reused by this feature.

Before writing, the store marks publication uncertain. Only a successful
publication clears it; false/exception/cancellation cannot make cached values
connection authority. Rejection, corrupt reads and busy storage have no active
worker effect. An explicit successful save can resolve publication uncertainty.
The private record contains numeric settings, never identity or consent.

Each Activity start controller admits one identity/original job, records it
before starting or registering completion, and releases only on that job's
identity-matching completion. After loading it rechecks cancellation and current
connection state. An old result cannot replace another current worker. Dialog
drafts contain eleven strings of at most twenty characters, retain no active
worker, and cap presented backend errors at 256 characters. Closing/disposal
cancels its request without pretending the original storage call has settled.

Conservation is one store API slot per admitted call and one start slot per
original controller job; rejected work allocates no backend operation. Eventual
release assumes the framework call returns and the owning dispatcher eventually
executes its completion. A stuck call keeps its funded slot and later requests
reject; a timeout, cancelled observer or reference count never substitutes for
that completion. The finite history model, real owned-thread fixtures, hosted
SDK adapters and full-suite counters complement one another; none establishes
unrestricted native device safety or battery behavior.
