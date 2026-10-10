# T834 cancellation-owned Wi-Fi attempts

A bounded native owner completes an acknowledged ADB connect and either commits
its route or disconnects a newly created route when its caller disappears.
Already-connected routes are preserved. The caller still owns its asynchronous
storage request: cancellation before commit rejects queued storage work; an
already-started atomic commit may finish and retain the reconnect preference.
Cleanup never rolls back a newer configuration write.

Attempt capacity and the in-process address lease remain owned through actual
native-thread join. Native adapters additionally retain a nonblocking filesystem
lease through command completion and cleanup, coordinating daemon and CLI
attempts. There are 64 stable lock stripes per user; hash collisions conservatively
reject simultaneous native attempts. External ADB clients do not honor these
Blent leases. Native commands retain their existing five-second deadlines and
bounded output/process retirement. Explicit finish waits at most six seconds;
expiration retains ownership and capacity until real completion.

Permanent red-before-green T834 coverage reproduces cancellation during the
post-connect storage snapshot. Additional tests cover a held commit, cancellation
during connect, held cleanup with sibling progress, same-address exclusion,
existing routes, bounded admission, invalid addresses, optional command ports,
native lock overlap and symlink rejection. All 329 host-library tests pass
(three existing ignored tests). All 62 scoped production functions meet individual
80% line coverage and complexity <=9. Four architecture guards pass. Windows GNU
cross-compilation passes; native Windows execution/counters remain T737.

`BLENT_WIFI_ATTEMPTS` defaults to 8, accepts 1..64, and counts active plus retiring
owners. No downloads, physical ADB operations, dependency changes or reloads.
