# T875 authenticated receive-window agreement

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Portable BLCP v1 contract defines a fixed 12-byte host proposal and tablet
selection, bounded to 4096..1048576 bytes. Both directions use the smaller
configured capacity; host rejects expansion, wrong roles, versions, lengths and
reserved bytes. Host exchange runs after mutual TLS, before initial CREDIT,
under one absolute deadline spanning write, flush and complete reply.

Offline validation: four host tests and three portable contract tests pass;
combined suite passes 69 broker tests and 332 common tests. All seven scoped
production functions pass the native per-function 80% LLVM gate; reviewed
complexity is at most 7. Retained tests include unequal capacities, fragmentation,
trailing-frame preservation, every prefix-byte mutation, bounded arbitrary data,
invalid local settings, partial I/O deadlines, closed streams and truncated reply.
Combined T874/T875 counters, inventory and source hashes are retained.

The adapter is not yet invoked by a production broker. That integration remains
T853; Android's matching adapter remains T854. Pairing/media authority still
requires current leases; failed or cancelled setup must retire its native owner.
No downloads, live service changes or tablet interaction were needed.
