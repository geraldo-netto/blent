# T853 Linux broker composition

The optional `blent-usb-broker` executable and on-demand client compose one
private per-user owner, read-only inventory, explicit selected-device startup,
persisted identity/trust, stock mutual TLS, authenticated receive windows and
bounded registered media bridges. Portable control policy remains independent
of Linux process, filesystem, descriptor and USB adapters.

Validation ran offline against synthetic helpers and isolated sockets. No live
USB access, installation, downloads or device-mode changes occurred. The normal
test suites retain the executable/election/restart test, actual mutual-TLS
registration/control-byte test, durable revoke and stuck-native retirement tests.
Production Android service, UI and debugging-off display acceptance remain
T854/T855/T857; this artifact does not claim a working production tablet display.

- 189 host library tests pass (`broker`, `accessory::setup`, `secure_connection`).
- Two real broker executable integration tests pass.
- Thirteen shared trust tests pass.
- All 185 scoped production functions have at least 80% executable-line coverage
  (minimum 81.82%). The non-Linux unsupported entrypoint is explicitly outside
  this Linux measurement; no Windows runtime support is claimed.
- All 471 reviewed production/test Rust functions have cyclomatic complexity ≤9.

Retained red logs cover truthful saved-trust publication, premature bridge
admission, repeated terminal task reads, inventory task retirement and cancelled
terminal waits, failed inventory reporting, route-retirement callback reentry,
and propagation of nondefault approval deadlines. Those permanent regressions
pass in the retained host run. Tests are not skipped or removed after fixing.

`manifest.json.gz` freezes the full source inventory; `sources.json` lists the
measured source hashes. `coverage.json` records exact scope and native counters.
LCOV and logs retain the native evidence. Commands use `CARGO_NET_OFFLINE=true`,
`--offline --locked`, `accessory-transport`, and the cached LLVM coverage target.

The shared trust export deliberately uses only the freshly executed
`blent_config-88e1a3137dc8aa58` test binary with LLVM's `export -format=lcov`,
using the same merged `blent.profdata`. Cargo's all-object export from the reused
target included stale generic source mappings and incorrectly counted comment
lines as uncovered. Both exports are retained: `common-current.lcov.gz` is used
by the passing gate; `common.lcov.gz` is diagnostic only. T880 tracks permanent
collection hardening. No threshold or source exclusion was changed to pass.
