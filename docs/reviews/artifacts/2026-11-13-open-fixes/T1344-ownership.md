# T1344 ownership evidence

One admitted route owner holds two endpoint states, one request slot, one original
worker/runtime and a retirement-pool admission. Native adapters also hold device
and local-port leases. Admission precedes thread creation. The worker alone
changes creation/removal state, using the existing independent temporary-mapping
policy. Preexisting mappings are never adopted merely because they match.

An unacknowledged creator remains owned across errors, empty snapshots, cancelled
waiters and facade loss. Native commands return only after their original child
and collector fences. Cleanup only retires a mapping after its original effect
is settled; the original worker then exits. Pool retirement observes that native
thread exit and destroys leases before publishing Completion. Neither a logical
stop nor an empty snapshot permits reuse. Physical identity change blocks cleanup
until the original tablet is accessible through a proven alias.

Cleanup liveness assumes original native calls return, the delegated effect is
acknowledged/observed, and the original tablet/removal eventually becomes available.
Unsettled outcomes and panic quarantine retain bounded funding and leases. No
unconditional eventual-cleanup claim is made. Linux monitor ticket retention is
separate T1346; Windows orchestration/native acceptance remains T1352.

The two initial regression failures retain their original decisive assertions.
The lost-acknowledgement test now awaits its independent publisher's notification
instead of asserting a transient mapping snapshot: successful eager cleanup may
already have removed that mapping before the caller resumes.

Validation: 50 shared USB tests, 6 Linux native-process route fixtures and 3
independent creation-policy tests passed. 45 changed production functions meet
80% executable-line coverage and complexity <=9. Coverage was exported from the
exact executed instrumented executable; stale unrelated cached executables were
not merged into its counters. Default-feature Windows GNU compilation passed;
accessory-feature compilation lacks MinGW, and native Windows remains unvalidated.
