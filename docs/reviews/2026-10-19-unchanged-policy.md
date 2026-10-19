# T1109: portable unchanged-frame policy

The common library now implements a one-second refresh policy. Native adapters
and user configuration remain T1110/T1111; production capture still uses its
existing cadence. Joined comparisons remain T1112 and parent T915 stays open.

## Contract

One policy instance belongs to one original encoder, named decoder, recipe and
continuous viewer membership. The adapter supplies read-only `Conditions` before
each observation and tick: that original source is live, viewers are present,
and all current original viewers negotiated transport liveness. These values
grant neither media admission nor native retirement. The adapter must validate
original identities using its existing immutable views and actual native proofs.

An explicitly selected suppression mode may construct this policy. An unknown
recipe or unnamed decoder starts rejected at compatibility cadence. Losing any
condition rejects the original instance permanently; later true observations
cannot revive it. Native request expiry, exact source identity, cleanup and
immutable session configuration remain adapter responsibilities.

The policy composes the existing authoritative T492 qualification state, retaining
one ACK, timestamp, keyframe and cost history. It first requires the existing
16-sample compatible 200 ms baseline and 16-sample 500 ms trial. After those
pass, it requests a **provisional** 1,000 ms cadence to measure an additional
16 consecutive refresh observations with PTS gaps in 850,000–1,250,000 us.
Only then is the new phase `Active`. Short motion gaps reset an incomplete
refresh trial; they preserve completed qualification when existing cost and
progress checks still pass.

The existing matched ACK limit, baseline cost margin, keyframe progress and
1.5-second proof expiry remain unchanged. Ticks use one monotonic clock;
backward ticks and future-dated ACKs reject. Delayed processing of a valid ACK
does not falsely imply a backward clock. Rejection clears qualification and
requests the existing 200 ms compatibility cadence. A new original session
must obtain new evidence.

For a current available frame, first publication, fresh damage, cursor changes
and explicit refresh requests always emit immediately. An unchanged picture
emits at the selected interval boundary or later. The one-second recovery
refresh preserves periodic keyframe/configuration opportunities; this policy
does not claim an indefinite stop or zero wakeups. Adapters must preserve fresh
wakeups and clear requests on viewer change, recovery, stop and source loss.

The public values contain no OS path, FIFO, concrete encoder, UI or worker type.
The policy owns one bounded existing cost vector (capacity 16), scalar trial
count and optional tick time. Samples are copied values; no native buffer,
callback or externally owned mutable view is retained. Rejection retains the
owned vector capacity until normal policy destruction; destruction frees it.
There are no retaining cycles, detached tasks or new native cleanup obligations.
Sample clock domains remain the existing signed PTS/unsigned host microsecond
domains. Checked gap arithmetic and a count capped at 16 prevent overflow.

## Evidence and limits

Ten permanent policy tests pass. An independent interval oracle checks all four
phases over elapsed values 0–1,100 ms and each change kind. Boundary/fault cases
cover recipe/decoder eligibility, all seven invalid condition combinations,
window endpoints, incomplete trial reset, motion after qualification, missing
keys, invalid timestamps/sequences, slow ACK/cost growth, exact expiry, backward
ticks and delayed ACK processing.

The bounded exhaustive test checks all six-event traces of length four from four
initial phases: 5,184 traces and 20,736 transitions. Its independent absorbing
rejection oracle catches an intentional ignored-original-authority fault. The
emission oracle catches an intentional fault delaying cursor/damage refreshes.
Both faults are restored before final verification; no assertion is weakened.
This finite model establishes its selected transition property, not unrestricted
native or hardware safety.

Fresh offline LLVM coverage runs all 380 common-library tests successfully, with
no failures or skips. All ten new production functions measure **100% executable
line coverage individually**. The initial sandboxed run fails two unchanged
SO_PASSCRED descriptor fixtures; the full final suite passes outside that
restriction. A separate sandbox probe confirms SO_PASSCRED is denied with EPERM. Source estimates cover 27 functions, maximum 7, using the repository
Rust parser; native SonarQube measurements remain T1019. Five architecture tests
pass. Raw counters, input hashes, failures and faulty controls are retained in
[the artifacts](artifacts/2026-10-19-t1109/README.md).

The tested CPU sparse-delivery scope is documented under
[T1107](2026-10-19-sparse-delivery.md). Physical decoder qualification, additional
encoder recipes/sizes and the in-process path do not follow from this portable
policy. Unqualified combinations must remain visibly compatible. No physical
display, suspend, tablet, installed app, network or battery action was performed.
