# T899 — retain the concurrent authentication regression

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The T282 fixture manually installed a WebSocket without the immutable connection
route captured by T856. Authentication correctly required that route, so both
API 27/34 cases failed before reaching the existing auth-entered latch.

The fixture now supplies its recording socket through the existing injected
factory and calls the normal connect path. Every auth-first, concurrent sender,
ordering, cancellation and thread-retirement assertion is retained unchanged.
The same two permanent test variants now pass offline. No production behavior,
timeout or assertion was weakened; no production coverage gate is changed.
