# T1031: original command and collector observations

Shared sync/async command adapters now accept independent completion observers.
Existing entry points delegate to that same execution path. Immediately after
collector creation they report its original thread-join proof; immediately after
child creation they report the original native-child admission. Spawn failure
still reports the already exposed collector. Cancellation, timeout or a returned
output value never substitute for thread/process retirement. Observer callbacks
must not block. Synchronous observer panic after exposure explicitly retires the
original child before resuming unwinding; async child ownership already supplies
that guarantee through its retained Drop path.

Five permanent regressions were added before forwarding and failed with missing
sync/async/failed-spawn/canceled/held-collector proofs. The same assertions pass
after forwarding. Additional normal-suite tests cover observer panic and three
unrepresentable deadline boundaries, which reject before native exposure. A real
held collector read survives parent cancellation and retains its original proof
until its actual thread exits. Existing input secrecy, byte limits, descendant
retirement and status regressions remain unchanged.

Fresh locked offline native collections pass 341 common library tests, all six
command-lifetime integration tests and the existing host reaper regression. Joined
counters cover all 32 scoped command/collector production functions individually
at >=80% (minimum 90%). New observer forwarding and synchronous dispatch reach
100%; asynchronous dispatch 96.77%. The host collection initially exported host
sources only; a second native report selects both existing workspace packages
from those same objects/profiles to expose common counters, without rerunning or
rebuilding. Both reports and their matching source/object attestations are
retained, along with each configuration's missing counters and joined results.

Unchanged repository Sonar-rule counting covers 75 production/test functions,
maximum 7, using cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2. This is a source
audit, not native SonarQube certification; T1019 remains. Windows native execution
and per-function counters remain unavailable under T737. Linux evidence does not
qualify Windows job/pipe branches.

```sh
python3 /tmp/blent-t1031-evidence/scoped_rust.py coverage-final
python3 /tmp/blent-t1031-evidence/scoped_command_lifetime.py coverage-command-lifetime
python3 /tmp/blent-t1031-evidence/scoped_host.py coverage-host-entry
# Export both packages from coverage-host-entry's same owned target/environment:
cargo llvm-cov report --lcov -p blent -p blent-config \
  --output-path /tmp/blent-t1031-evidence/coverage-host-entry/common-host.lcov
python3 /tmp/blent-t1031-evidence/merge.py
```

Native objects remain in the exclusively created evidence target trees. All
work used installed tools/caches, with no downloads, installs, transfers, pushes,
live display changes or application updates. Capture propagation remains T1032;
idle integration and joined auxiliary acceptance remain T1029.
