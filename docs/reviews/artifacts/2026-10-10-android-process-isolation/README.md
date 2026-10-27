# T809: on-demand native worker isolation

On-demand Android workers isolate independent media owners and wait for confirmed worker death before replacement. Native IPC trials and hosted tests validate supervision, with extra process-memory cost and no shared-driver fault-containment guarantee.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Maintainer selected separate workers on 2026-10-10, prioritizing isolation.
Five private, nonexported processes own display, camera, microphone, speakers
and metadata independently. Main-process owners retain consent, foreground
services, UI and lifecycle policy. Workers are bound only on demand, terminate
on unbind/client death, and never automatically replay a failed request.

`MediaTask`, `WorkerEvent`, `LifecycleRun` and `DisplaySession` are independent
contracts. Android Binder, Surface transfer and native factories are boundary
adapters. Control IPC carries requests, bounded observations and Surface handles;
pixels and PCM stay on the existing worker-owned sockets/native paths. No raw
frame Binder copies. Each process admits one native owner. Replacement waits
for confirmed death; missing exit remains quarantined. Stop schedules native
cancellation separately from a 500 ms exit grace and a 3 s death deadline.
This isolates application process faults; it does not promise containment of
shared camera/audio services, drivers or kernel hangs.

## Measurements and native acceptance

`trial-0..3.json` and prototype provenance compare separate versus grouped
workers. Four separate idle workers cost about 90 MiB PSS versus 23 MiB for a
single shared process. RSS includes shared pages and must not be summed as
physical memory. Prototype cold startup was about 0.4 s per worker. Production
full debug app cold startup is higher; see `production-final/production-*.json`.
No throughput/latency improvement is claimed from isolation. No permanent idle
workers remain after resource retirement.

The permanent device runner executes two production-IPC trials, kills a disposable
worker, holds native execution and retirement, checks sibling progress, explicit
reconnect, Surface validity, real native metadata and parent-death cleanup. The
factory alone substitutes fake held media resources; production IPC/control and
retirement code remain byte-identical. `production-final/provenance.json` pins
source/APK hashes and substitution. Test grace is 100 ms, versus 500 ms production.
This validates process supervision, not physical camera/microphone/audio quality;
T718/T720 retain their physical acceptance scope. No production app/host reload.

Run offline with an attached disposable-test device:

```sh
python3 scripts/benchmarks/worker-project.py --directory /tmp/blent-worker-tests
python3 scripts/benchmarks/worker-native-tests.py \
  --adb /path/to/adb --serial DEVICE --project /tmp/blent-worker-tests \
  --output /tmp/blent-worker-results
```

Only `com.blent.workertest` is installed; runner removes it in finally. The
production package is never stopped, replaced or uninstalled. No downloads.

## Permanent automated regressions

Normal Android unit suite retains manifest/process separation, independent
contracts, bounded malformed/deep JSON fuzzing, mailbox byte/count limits,
generation rejection, Binder failures, held retirement, Stop/EXIT, ownership
completion, callback failure, canceled admission and manual restart regressions.
`WorkerContractTest`, `AndroidWorkerRunTest`, `MediaWorkerServiceTest`,
`RemoteDisplayTest`, `RemoteAdaptersTest` and existing owner/UI tests cover the
production boundaries. The three final cleanup regressions first failed on
receipt invalidation, cancellation skipping retirement and failed death-unlink
bypassing termination; all pass after fixes. Architecture source guards reject
native dependencies in portable contracts and main media owners.

Full offline suite: 902 tests, zero failures. All 1,081 measured production
methods meet individual 80% executable-line coverage; no method exceeds
cyclomatic complexity 9. Full counters and red/green logs are retained here.

</details>
