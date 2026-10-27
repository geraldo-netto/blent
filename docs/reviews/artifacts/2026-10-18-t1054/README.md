# T1054: native fixture trace publication

The native fixture records the original request before sending an acknowledgement. Deterministic trace tests fix the diagnostic race without changing production behavior.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T1009's installed SDK capture run exposed the existing bad-resume fixture racing
its diagnostic trace: the helper sent an ACK before appending its request record.
The host could reject the forged ACK and kill the original helper before logging,
so the retained assertion observed zero records rather than one. The trace also
recorded the forged reply bytes instead of the original received request.

The private helper now snapshots the original bounded request, flushes and closes its
append record and then sends the reply. A held-record regression deterministically
proves no ACK precedes recording; it failed with the original send-before-record
ordering. A second permanent regression rejects a forged reply while preserving
original request identity; it failed before received bytes were retained. Both
runtime red logs and the original SDK failure are retained. All tests remain in
the normal suite. CLI and installed network-disabled SDK each pass both new
regressions plus the unchanged bad-resume test. No assertion or deadline weakened.

Each new fixture owns one helper, original child completion, task and control
exchange. The task owns the manager through operation and cleanup; the observer
borrows only an atomic ACK observation. The recording barrier is released before
asserting the oracle, then shutdown and exact helper child/stdout restart fences
settle before the private root is released, including the faulty control. One
control read admits at most 32 bytes; each trace record has at most 64 hex bytes
plus newline. The selected regression domain uses one request and bounded 64x32
frame storage. No native acquisition/retirement authority comes from a trace row.

Production code is unchanged. Repository source audit covers all three changed
Rust functions, maximum cyclomatic score 7, using cached tree-sitter-rust 0.24.2 /
tree-sitter 0.25.2; native SonarQube remains T1019. These test-only functions need
no production line-coverage counters. No downloads, installation, upload, push,
physical display, lock/suspend or tablet work occurred. T1009's final collection
will use a fresh source/object inventory after this fixture correction.

</details>
