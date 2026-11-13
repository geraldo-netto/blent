# Linux lifecycle recheck — 2026-11-13

Reviewed source HEAD `9f8660aa3f0b9c01f78142268166947c2f50f6ff`.
Review only: no production/test edits, service/device changes, physical suspend,
driver changes or configuration changes. Parent records findings in TODO.md.

## T1418 — delayed observer owner return becomes permanent unsupported state

Severity medium; effort small. Reproduced using the public `PowerFactory` and
`Following` against an isolated private D-Bus, with a power-only logind fixture.

1. Original login1 owner publishes supported power observation in epoch 1.
2. Release its name and leave it absent through the first replacement attempt
   (probe source deadline/backoff 100 ms).
3. The replacement publishes epoch 2 with `power=false`, display/auxiliary demand
   both true, and `source_lost=false`.
4. Recreate login1 and emit `PrepareForSleep(true)`. After another 350 ms the
   publication remains unchanged. There is no actor watching for its return.

Exact output: `delayed-owner.log`; executable probe source: `delayed_owner.rs`.
The finite observation supports the symptom; the source establishes why no
further recovery is scheduled: `host/src/activity.rs:153` discovery catches
`Actor::logind` creation failure as unsupported, and `:194` starts a source with
default capabilities when no actors were discovered. In
`host/src/activity_follow/recovery.rs:49`, successful creation enters following;
`host/src/activity_follow.rs:162` retries only when polling fails. This empty
source has no backend actor to observe owner return or fail its polling.
T1413's original loss notification at `activity/native.rs:270` works, but its
single replacement can therefore end observation permanently.

Consequences: a service restart spanning the replacement attempt can disable
future suspend observation until another backend failure or daemon restart,
and renew media demand without fresh observation of the previously supported
power source. A genuine unsupported startup remains an intentional distinct
contract; changing capability sets between valid fresh epochs is not itself a
bug. Fix discovery/recovery so transient absence of an observed capability
retains conservative demand and bounded retry/name watching, preserving original
native settlement and explicit unsupported startup. Permanent regressions should
cover delayed owner return, prolonged absence, fresh observations and explicit
stop without recreation.

## Checks and ownership boundaries

Examined native activity discovery/loss/recovery, feed/source contracts,
replacement epoch policy, capture pause/supervision, helper restart receipts,
FIFO identity ownership and native retirement accounting. No additional
confirmed issue emerged in these scoped paths.

- Existing T1413 private-bus regression selection: 4/4 tests passed
  (`activity-regressions.log`).
- Existing T1415 isolated pause-recovery selection: 6/6 tests passed
  (`capture-regressions.log`). These exercise invalid ACK/EOF, renewed demand,
  original helper cleanup, cancellation and cancelled reader generation.
- Capture error recovery awaits encoder generation/reader retirement before
  accepting strict pause failure; helper replacement additionally requires both
  child and stdout cleanup receipts. FIFO ownership pins original inode identity
  and prevents cleanup from unlinking a substituted inode. Completed retained
  native receipts are pruned while unsettled obligations remain owned.
- Probe explicitly stopped Following, awaited actual retirement, closed both
  D-Bus connections and exited its private `dbus-run-session`. Process inspection
  after execution found no remaining probe/private bus/test process.
- Probe complexity measured by the project syn 2.0.119 frontend: all 10/10
  authored functions measured, maximum 3 (`probe-complexity.json`).

These are source and isolated fixture observations, not physical EVDI, USB,
suspend/resume or platform acceptance. No new coverage campaign was run.

## Reproduction metadata

Used existing instrumented project artifacts from the preceding verified fixes;
no production compilation or replacement of the running application was needed.
Rust dependencies must match the library's fingerprint; arbitrary newest rlibs
can select a different Tokio crate instance. From the repository root, rebuild
the external probe with this Python command body (no repository build output):

```python
from pathlib import Path
import subprocess
root = Path.cwd()
out = Path('/tmp/blent-lifecycle-recheck-linux')
out.mkdir(exist_ok=True)
source = root / 'docs/reviews/artifacts/2026-11-13-lifecycle-recheck/linux/delayed_owner.rs'
args = ['rustc', '--edition=2021', '-C', 'instrument-coverage', str(source),
        '-o', str(out / 'delayed-owner')]
artifacts = {
    'blent': 'blent/c16265474e6be6f1/out/libblent-c16265474e6be6f1.rlib',
    'blent_config': 'blent-config/e8fb2a5f0cf99980/out/libblent_config-e8fb2a5f0cf99980.rlib',
    'zbus': 'zbus/240d4e1d9a0efdaf/out/libzbus-240d4e1d9a0efdaf.rlib',
    'tokio': 'tokio/9a9d4a755c6dd115/out/libtokio-9a9d4a755c6dd115.rlib',
}
for name, relative in artifacts.items():
    args += ['--extern', f'{name}={root / "target/debug/build" / relative}']
for path in (root / 'target/debug/build').glob('*/*/out'):
    args += ['-L', f'dependency={path}']
args += ['-L', f'dependency={root / "target/debug/deps"}']
subprocess.run(args, check=True)
```

```sh
dbus-run-session -- env LLVM_PROFILE_FILE=/tmp/blent-lifecycle-recheck-linux/owner-%p-%m.profraw /tmp/blent-lifecycle-recheck-linux/delayed-owner
env LLVM_PROFILE_FILE=/tmp/blent-lifecycle-recheck-linux/lib-%p-%m.profraw target/debug/build/blent/4a4e48b452121ffa/out/blent-4a4e48b452121ffa activity::tests::recovery --nocapture
env LLVM_PROFILE_FILE=/tmp/blent-lifecycle-recheck-linux/capture-%p-%m.profraw target/debug/build/blent/66c07be479eae2be/out/blent-66c07be479eae2be capture::activity_tests::pause_recovery --nocapture
```

Artifact SHA-256 values:

| Artifact | SHA-256 |
| --- | --- |
| blent c16265474e6be6f1 library | c13539487c3400219c24880077e142dc7374d33e9c3edc276e5f64caa5c7da3a |
| blent 4a4e48b452121ffa test binary | d527c33bd27c86c565a197aac622fcdeb4da993449b19613a45530256bade696 |
| blent 66c07be479eae2be test binary | 7667e2d2d070ce3439e617ca5ce747ef849693b6b24a4394cd93ff2a00707563 |
