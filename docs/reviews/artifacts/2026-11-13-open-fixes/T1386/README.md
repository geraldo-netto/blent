# T1386 — reject malformed inventory serials

Both permanent regressions fail before the fix and pass afterward. Inventory
records use the independent serial validator already used by tablet contracts.
C0/C1 controls, oversized ready/nonready serials, maximum UTF-8 byte length,
Unicode, network and mDNS values are covered. A malformed observation preserves
the attached session, its credential and its authenticated ping/pong connection.

The changed record function measures 100% executable-line coverage and complexity
2 with cargo-llvm-cov 0.8.7 and the project syn 2.0.119 frontend. The validator's
body is unchanged; only its visibility changes. Borrowed validation has no
retained allocation or native effects. Existing monitor ownership is unchanged.
