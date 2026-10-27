# Per-tablet display controls (T1274)

Per-tablet display Start/Stop binds commands to the original daemon and tablet identity while leaving camera, audio and the connection independent. Accepted Stop means intent changed; capacity remains occupied until actual cleanup. Linux contract and IPC tests passed, and the later Start-scope fix preserves siblings at capacity; Windows display control remains unsupported.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

Implemented owner-selected variant B. Linux daemon accepts Inspect and targeted
Start/Stop using T1272 contracts. Workspace wiring is T1273. No application was
installed or running tablet disturbed during validation.

Stop validates the daemon lifetime nonce, tablet identity and exact original
slot/generation before changing intent. It suppresses only that tablet's display
admission and closes its existing ticket. ADB uses the existing route/session
retirement workflow; direct USB uses the existing connection/session owner. The
underlying tablet connection, camera, microphone and speakers are independent.
Stopping the primary does not promote/restart a healthy extra tablet.

Capacity remains occupied through unpublished startup, cancellation and native
retirement. Start cannot revive a closing lease. A stale Stop cannot act on a
replacement, including the same tablet in a reused slot. Acknowledgement certifies
accepted intent, not native retirement or video delivery. Assigned certifies a
published display assignment; it does not certify streaming frames.

Stopped intent survives disconnect/reconnect within this daemon lifetime. A daemon
restart resets it and retains existing automatic display discovery. Global service
administration stays separate. When the service is stopped, per-tablet controls
require starting that global service first; they never silently start every tablet.
Windows display/input remain unsupported.

Linux transport uses a user/PID/start-time-bound abstract Unix socket and checks
peer credentials. Abstract endpoints disappear with their original listener;
there is no pathname deletion/reuse race. The client additionally validates PID
and start time. Commands and responses have a 32,768-byte limit and a two-second
I/O deadline. An unconfirmed client outcome requires inspection, not blind retry.

`BLENT_DISPLAY_CONTROL_CLIENTS` selects 1..64 concurrent IPC jobs (default 8).
Higher capacity tolerates more concurrent status clients, using more descriptors
and bounded frame buffers. Admission closes while this pool is full. The daemon
owns at most four display slots and 1,024 explicit tablet policy exceptions. Exhaustion rejects
without releasing or overwriting an existing owner.

Monitor owns all native effects after admission, including caller disconnect.
Server read/write tasks own bytes and streams only. Monitor cutoff consumes the
server, aborts and joins its jobs, then performs original native cleanup. The client
owns a current-thread runtime and socket until reply/error; cancellation cannot
retire admitted daemon work. Reuse requires the original native completion proof.
Existing native retirement/funding contracts are preserved.

The later [T1275 scope correction](2026-10-27-display-start-scope.md) makes manual USB Start cooperative at capacity and derives ADB eligibility from the same plan.

Offline Linux evidence for the original T1274 commit:

- Two scope/capacity regressions failed against the initial no-op policy, then
  passed unchanged. Permanent tests also cover stale daemon/lease identity,
  bounded/invalid intent, independent USB eligibility, caller loss after sending,
  native IPC credentials/frame limits/cancellation/rebinding, capacity settings,
  failed ledger publication and original retirement.
- Full instrumented run: host library 1,022 passed / 3 existing ignored; daemon
  603 passed / 4 existing ignored; common 419 passed. Final targeted additions
  passed with cumulative coverage; no ignored test was changed or added.
- All 38 changed production functions measure >=80% executable-line coverage
  individually (minimum 80%). Rust 1.98.1, cargo-llvm-cov 0.8.7, LLVM 22.1.8.
- Locked syn 2.0.119 frontend: 128 functions across changed modules and their
  regression files, maximum cyclomatic complexity 9.

Source-hash manifests, failing/passing logs and LCOV remain in the owned external
work directory `2026-10-26-t1267-t1270`. These finite fixtures establish the named
boundaries, not unrestricted native safety. Existing physical mapping acceptance
and native Windows blockers remain visible in TODO.md.

</details>
