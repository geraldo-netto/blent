# T929 optional ADB diagnostic

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Native USB builds describe missing ADB as unavailable optional automatic discovery,
and explicitly state that direct USB does not require ADB. Default builds retain
the required-ADB install/restart diagnostic. Inventory behavior and retry cadence
are unchanged.

The permanent re-executed diagnostic test failed before the message fix and passes
with and without `accessory-transport`. Its missing-command fixture never accesses
a device. Exact fresh native/default test mappings provide individual coverage for
`queue_inventory` (at least 80%); all 30 reviewed functions have cyclomatic
complexity at most nine. Broader unchanged function coverage is not claimed here.
No downloads, installs, reloads or physical tablet tests were performed.
