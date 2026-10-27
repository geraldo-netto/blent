# T926 Windows GUI fixture compilation

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The normal Windows test target failed to compile because its existing App fixture
omitted the T894 connection panel. Initialize that panel using its established
Default implementation. The existing t495 preview rendering regressions remain
permanently in the normal Windows suite; their ordinary compilation is the
regression guard for this compile-time fixture defect. No production behavior or
coverage exclusion changed.

Red and green evidence use the same offline command:

```sh
CARGO_NET_OFFLINE=true cargo check --offline --locked --target x86_64-pc-windows-gnu -p blent -p blent-gui --tests
```

Native Windows execution is unavailable and remains tracked separately. Successful
cross-target checking establishes compilation only.
