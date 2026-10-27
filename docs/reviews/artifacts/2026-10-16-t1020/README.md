# T1020: retain original helper startup cleanup

Helper startup now records original child completion immediately after spawn. Canceled or malformed handshakes cannot admit a replacement while dependent cleanup remains held.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Helper startup records the exact child completion immediately after spawn,
before stdout extraction or the asynchronous card handshake can fail or be
canceled. The certificate remains in `pending` until native cleanup completes;
replacement rejects before FIFO creation or another spawn. Successful startup
transfers ownership into `child` and clears the pending marker. Existing process
reaping, FIFO leases and ordinary shutdown stay authoritative.

Two permanent normal-suite regressions fail before the fix with
`T1020 startup lost original retirement certificate`. They exercise a canceled
silent handshake and an invalid card report. Each real child is reaped, then an
original dependent cleanup lease is deliberately held. The replacement spawner
must not run until both the retained and independently observed original
certificates complete; releasing that cleanup permits a fresh successful startup.
An injected private native spawner follows the existing encoder startup pattern.
No tests, assertions or retirement requirements were removed or weakened.

Normal locked/offline host library and binary collection passes 752 library and
443 binary tests. Seven pre-existing benchmark ignores remain unchanged. A
separate fresh SDK collection passes ten optional encoder helper tests, including
both regressions and shared-raw socket preparation. Combined immutable native
LCOV covers all 21 helper production functions at the individual 80% threshold.
The startup wrapper reaches 100%, and `start_with` 97.9%. Neither individual
configuration supplies every helper counter; only the joined result passes.
Both manifests, native-source hashes, owned-object attestations and raw LCOV
remain separate and are retained here.

The optional configuration uses the installed Freedesktop SDK 25.08 with network
explicitly disabled, installed host libclang 18 and its builtin headers, and
narrow temporary links to existing dependent libraries. Initial diagnostic runs
exposed missing libclang and then its builtin-header path; those failed runs do
not contribute counters. No tools, packages or libraries were downloaded or
installed. `sdk-tools/cargo` is retained compressed as `cargo.gz`.

The scoped source audit measures 45 helper/test functions with maximum complexity
8, using unchanged repository counting definitions and the cached pinned Rust
parser. This is not native SonarQube evidence; T1019 retains that gap. Source
versions appear in `tools.json`. Native objects remain under the exclusively
created `/tmp/blent-t1020-evidence/coverage-cli/target` and `coverage-sdk-c/target`.

```sh
cargo test --offline --locked -p blent --bin blent t1020 -- --nocapture
python3 /tmp/blent-t1020-evidence/scoped_rust.py coverage-cli
PATH=/tmp/blent-t1020-evidence/sdk-tools:$PATH \
  python3 /tmp/blent-t1020-evidence/scoped_sdk.py coverage-sdk-c
python3 /tmp/blent-t1020-evidence/merge.py
```

The configuration-only checks intentionally report their individual missing
counters; `merge.py` enforces the combined per-function threshold without new
exclusions. Retained monitor pause is still T1021 and activity composition T1015.
No live display, tablet, suspend, service or boot configuration changed.

</details>
