# T825 asynchronous monitor inventory

Windows settings redraw consumes an immutable published snapshot. Native monitor
queries run in a separate admitted worker, independently of the existing service
status worker. Polls occur every two seconds; snapshots older than six seconds
are explicitly stale. Initial/error states are unavailable, while the last good
snapshot remains readable. Removed monitor selections remain visible and intact.

The worker holds no publication mutex while querying the OS. Dropping its owner
wakes idle waiting and rejects late publication; a stuck native query retains its
capacity through actual thread join. BLENT_MONITOR_WORKERS defaults to 1 (1..64).
This bounds replacement workers without blocking redraw or unrelated status work.

The permanent source-boundary regression fails before the fix. Permanent native-
independent tests exercise 1,000 reads during a held inventory, counted probes,
topology replacement, old snapshot immutability, stale/error/recovery states,
capacity retention, drop during query and idle wait, and removed selection UI.
85 GUI tests pass; all eight scoped production methods meet individual >=80%
line coverage and complexity <=9. The changed App constructor also stays <=9.
Windows GUI tests cross-compile offline. Native Windows enumeration timings,
runtime acceptance and platform-wiring counters remain explicitly under T737;
no native Windows speedup or complete platform coverage is claimed.

No downloads, live display changes or dependency changes. InventoryResult simply
names the existing independent monitor contract result type.
