# T874 private broker ownership and asynchronous IPC

Linux owner election holds a stable 0600 lock under an owned 0700 directory,
pins the directory descriptor through bind/cleanup, rejects unexpected stale
paths and removes only its own socket inode on drop. Async packet I/O has one
reader and one writer, bounded descriptor transfer and cancellation-safe packet
ownership. Shutdown wakes pending I/O but retains the descriptor until drop.

Offline native validation: 69 broker tests pass, including 13 new owner/async
cases for competing ownership, unsafe paths, renamed directories, replacement
sockets, descriptor lifetime, spurious readiness, stalled clients and cancellation.
All 21 scoped production functions pass the per-function 80% LLVM gate.
Combined T874/T875 reports are retained with source hashes and inventory;
complexity is at most 7. The spurious-readiness test also covers the previously
unmeasured WouldBlock adapter path. No production service, device or USB mode
was changed; no downloads. T853 still owns executable/client integration.
