# T1017: activity assembly convenience belongs to the fixture

Move LinuxSource::on_buses into the existing test module. Production connects
through the real discover/start boundaries; normal-suite fixture callers still
exercise those same boundaries. No maintained runtime path, public contract or
permanent regression is excluded, suppressed or changed. The pre-change compile
log (retained from T1012) reports the unused private method; current offline
compilation succeeds without warnings. This is a responsibility refactor with
no behavioral bug or artificial regression test.

Fresh locked/offline collection passes 19 host and 19 common selected activity
tests. All 93 production functions in common/src/host_activity and host/src/activity
pass individual 80% executable-line coverage; none missing. The fixture helper
itself remains in the normal test suite and whole-project complexity inventory.
The cyclomatic gate passes 22,225 functions with none above 9. Workspace formatting
passes; production/build source bytes remain immutable during native collection.

```sh
cargo check --offline --locked -p blent --features accessory-transport
/tmp/blent-t1007-venv/bin/python scripts/coverage/rust_collect.py --output /tmp/blent-t1017-coverage --package blent --package blent-config --lib --filter activity --feature accessory-transport
/tmp/blent-t1007-venv/bin/python scripts/coverage/report.py check --manifest /tmp/blent-t1017-coverage/manifest.json --lcov /tmp/blent-t1017-coverage/rust.lcov --native-sources /tmp/blent-t1017-coverage/native-sources.json --rust-platform linux --scope common/src/host_activity --scope host/src/activity --output /tmp/blent-t1017-coverage/per-function.json
cargo fmt --all -- --check
/tmp/blent-t1007-venv/bin/python scripts/complexity/check.py
```

No native application/service/device mutation or downloads. Physical activity
acceptance remains T914; this fixture refactor establishes no runtime support.
