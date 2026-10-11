# T942 separate resource retirement proof

ResourceLane preserves its existing execution-ended callback and separately
publishes actual successful retirement after the run and job body finish. An
unopened request publishes NOT_STARTED when superseded, stopped, rejected by an
inactive scope, cancelled before dispatch, or when its lazy factory fails. Lazy
factories must construct wrappers without acquiring native resources. No failed
retirement or timeout publishes actual-retirement proof. Quarantined pending
requests were never opened and may safely publish no-start proof.

Observer Exceptions are contained without skipping cleanup, duplicating proof or
losing the next request. The original owner remains installed through callbacks;
admission also checks for an existing owner after recovery reporting. This prevents
reentrant observers from admitting over a retiring replacement. VM Errors remain
propagated; the remaining Error ownership gap is explicitly tracked by T947 before
capture integration, rather than included in these Exception-path claims.

Permanent ended-observer failure is red against the old behavior before its fix.
Retirement feature tests are red after adding only the optional signature/schema,
before wiring notifications. The recovery observer regression is re-proven red
with an actual started replacement, then passes with the admission guard. Draft
fixtures assumed Unconfined dispatch and completion-handler ordering; final fixtures
wait for actual worker start and the serialized completion handler. Job.join alone
may observe completion before handlers finish. Assertions are retained and augmented
with owned-child counts. No product bug is claimed from those fixture assumptions.

All 934 offline resource, connection, camera, audio/display and worker tests pass.
All 23 scoped production methods meet individual native JaCoCo 80% line coverage;
all three authored constructors pass. All 38 reviewed functions have cyclomatic
complexity at most nine. A seeded 256-step replacement/stop property retains every
request's exact once-only proof without opening pending native owners. Existing
invalid deadline, delayed/failed cleanup and independent-lane tests remain.
Native counters, hashes, inventory and red/green evidence are retained. Scope is
ResourceLane, not whole-project coverage. No native hardware or USB acceptance,
downloads, installation or device changes. Camera capabilities remain disabled;
T947/T943/T944/T932 supply remaining composition and T858 physical acceptance.
