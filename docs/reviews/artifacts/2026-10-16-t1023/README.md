# T1023: acknowledged capture activity contract

The independent portable record contains a nonzero, monotonically increasing
big-endian u64 sequence and one strict boolean byte. Requests and acknowledgments
are exactly nine bytes. Sequence exhaustion fails without wrapping. The native
peer may echo a record only after that transition completes; the proof applies
to capture activity, independently of encoder and process retirement.

The Linux adapter uses a private nonblocking Unix sequenced-packet socket. It
retains the original request and send status across canceled futures. A later
call first finishes that exact transition, then submits its own transition when
needed. Malformed, foreign, truncated, descriptor-bearing and closed peers fail
the controller permanently. No timeout creates an acknowledgment. An opaque
socket lease can survive with the original process without sharing mutation
authority. Exec inherits only its reserved endpoint above stdio, using existing
owned packet transport and asynchronous readiness conventions.

Nine permanent normal-suite tests cover fixed independent vectors, bounded
length/flag/sequence mutations, wrap rejection, canceled sent and unsent commands,
socket backpressure, exact acknowledgments, terminal errors, delivered descriptor
cleanup, resource leases and exec handoff. Fresh locked offline native collection
passes all 334 common library tests. Every one of the 15 added production
functions exceeds the individual 80% requirement (minimum 90.9%); no exclusions
were added. The first diagnostic collection exposed missing parent-side success
coverage for descriptor inheritance, which the retained tests now provide.

The unchanged repository source-counting definitions and cached pinned Rust
parser report 29 authored production/test functions, maximum complexity 6.
This is an independent source audit, not native SonarQube evidence; T1019 retains
that gap. Compressed manifests, native object/source attestations, raw LCOV,
per-function counters, audit script and versions are included here. Native
objects remain in `/tmp/blent-t1023-evidence/coverage-final/target`.

```sh
python3 /tmp/blent-t1023-evidence/scoped_rust.py coverage-final
```

This prerequisite adds the contract and transport. Native capture integration
remains T1021, and application activity ownership remains T1015. No live display,
tablet or system configuration changed; no network transfer occurred.
