# T1155 — admit canonical peer length before reading

Private IPC validates the exact 64-byte peer field before reading or allocating its payload. Prefix-only and malformed-byte regressions passed without changing valid wire bytes.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The private IPC peer decoder used `readUTF`, which accepted a declared field up
to 65535 bytes before fingerprint validation. The received packet was already
bounded; this was an avoidable bounded allocation/read. It now admits the exact
64-byte length before allocating/reading 64 ASCII bytes and parsing the canonical
lowercase hex fingerprint. Existing valid `writeUTF` bytes are unchanged.

The permanent prefix-only input throws on any payload read. Before the fix the
regression fails on that read; afterward invalid lengths reject without touching
payload. Its unchanged whole-file hash and red result are retained. The bounded
corpus includes endpoint lengths and 1024 deterministic 16-bit lengths, every
truncation of a valid field and all 256 values at one fingerprint byte, alongside
the existing flat event/control/display/camera/audio codec suites. Invalid bytes
must reject; their particular exception class is not part of that payload oracle.

The decoder borrows a caller-owned stream for the synchronous call, allocates
only a fixed 64-byte temporary and immutable parsed result, and retains no stream,
callback, native resource or mutable authority. Failed reads publish no result.
There is no asynchronous cleanup or allocator lifecycle to add.

All eight scoped production functions/native variants pass JaCoCo 0.8.15 at least
80% individually. Cached Kotlin 1.9.20 PSI reports 21 production/test entries,
maximum 4; native SonarQube remains unavailable under T1019. The source snapshot
also contains unfinished T1151 work, which this scope does not qualify. No device,
installation, network transfer or push occurred. Raw local evidence:
`/backups/disk2/blent-work/2026-10-20-open-items/t1155`.

</details>
