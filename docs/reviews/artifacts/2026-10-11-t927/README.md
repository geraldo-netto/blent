# T927 portable USB connection coordination

Portable USB coordination reserves capacity before startup and destroys retained listeners before reuse. Discovery failure still advances cleanup, while stale or ambiguous intent cannot preempt working owners.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

`display_connections::Connections` joins the existing discovery, capacity and
registered-session owners without native handles, capture implementations or UI.
Complete authenticated snapshots provide admission intent. Ambiguous new peers
never trigger ADB preemption; already admitted exact links remain owned. Newest
eligible peers progress independently, including when another startup retries.

Each admission reserves a ticket before native startup. Composition retains the
opaque admission while its independently progressing startup runs. Startup failure
may release it only with proof of no exposure and complete cleanup. Binding is an
infallible ownership transfer, including authority lost during startup. Polling
invalidates cancelled/stale startup tickets and closes exact obsolete sessions.
Retirement destroys listener capabilities before making capacity reusable.

Retry deadlines are absolute and bounded to 60 seconds. Exhausted clocks cannot
spin native startup or preempt another working display. Ports are unique, nonzero
and limited to four slots. Discovery failure still processes owned cleanup;
registration failure does not close unrelated sessions. Closing never reports
retirement while a startup, session or discovery worker remains owned.

The permanent external-capacity-cancellation regression failed before its fix:
the draft continued publishing an Active session after its ticket closed. The
same test now passes. Nine normal-suite tests cover ambiguity, multiple peers,
stale tickets, cancellation, both cleanup orders, failure isolation, port/capacity
bounds and exhausted clocks. All fourteen T925 tests remain unchanged and pass;
shared test adapters now have one owner instead of duplicated implementations.

All fifteen production functions meet individual 80% executable-line coverage;
all 66 reviewed functions have cyclomatic complexity at most nine. Coverage uses
the exact fresh native test binary. Offline Windows production/test checking
passes; this is not native Windows USB support. Linux daemon/startup composition
remains T908 and physical joined display/touch acceptance remains T857.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --lib --lcov --output-path /tmp/blent-t927.lcov -- t927
CARGO_NET_OFFLINE=true cargo test --offline --locked -p blent --features accessory-transport --lib t925
CARGO_NET_OFFLINE=true cargo check --offline --locked --target x86_64-pc-windows-gnu -p blent -p blent-gui --tests
```

No downloads, installation, live broker/device changes or physical acceptance.

</details>
