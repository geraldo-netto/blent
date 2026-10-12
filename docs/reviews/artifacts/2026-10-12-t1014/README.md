# T1014: activity snapshots retain host stop fences

Independent ActivitySnapshot/ActivityTarget contracts define bounded source/fence
consistency and conservative original-worker invalidation. Feed projects accepted
ordered Policy observations into snapshots retaining the latest display and
auxiliary host stop stamps. A coalesced pause/resume cannot revive original work.
Lock/display-off affect display; machine unavailability affects each resource.
Viewer/content inputs remain separate, and no snapshot grants acquisition.

Whole-source loss makes supported observations unknown and rejects further events
for the original feed. Unsupported capabilities preserve existing behavior.
Invalid/stale/foreign observations reject atomically; source loss does not invent
a native revision. Gate still requires independently retained actual completion
and current original authority before replacement.

The feature regression initially fails because Feed/ActivityTarget do not exist.
Five permanent normal-suite tests pass after implementation, including actual
watch coalescing combined with a Gate and held retirement proof. They verify that
resume waits for cleanup and rechecks original authority. Bounded properties cover
all eight capability masks, all eight preference combinations and all 36 native
state combinations (2304 inputs), plus foreign/future/zero source stamps, maximum
revisions, inconsistent snapshots, source loss and late-event rejection.

Final fresh locked/offline collection passes all 314 common library tests. All 35
scoped production functions pass individual 80% executable-line coverage; none
missing. Workspace formatting passes. Whole-project cyclomatic checking passes
22,225 functions with none above 9. Production/build sources remained immutable
from snapshot through native counter checking; no diagnostic counters were mixed.

```sh
/tmp/blent-t1007-venv/bin/python scripts/coverage/rust_collect.py --output /tmp/blent-t1014-coverage --package blent-config --lib
/tmp/blent-t1007-venv/bin/python scripts/coverage/report.py check --manifest /tmp/blent-t1014-coverage/manifest.json --lcov /tmp/blent-t1014-coverage/rust.lcov --native-sources /tmp/blent-t1014-coverage/native-sources.json --rust-platform linux --scope common/src/host_activity --output /tmp/blent-t1014-coverage/per-function.json
cargo fmt --all -- --check
/tmp/blent-t1007-venv/bin/python scripts/complexity/check.py
```

T1015/T1016 implement capture and USB registration composition; T1009 retains
joined automated host acceptance, T1010 auxiliary integration and T1011 Android
handling. T914 retains physical acceptance in an owner-approved window. This
contract is not active daemon integration or a power/performance claim. No device
interruption, native platform mutation, downloads or deferred work occurred.
