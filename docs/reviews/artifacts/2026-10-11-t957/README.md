# T957: isolation probe control/operation separation

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Keep UID/interface/size checks, death/unblock commands and one-operation admission
on the Binder boundary. Move unchanged stall/surface/response work to the existing
worker thread's operation method. Preserve busy publication, response order,
finally cleanup and native Surface ownership. Cached compiler/API 34 compile the
exact source; all six functions score <=9 (control 6, operation 7).
Benchmark coverage exemption retained. No device trial or isolation gain claimed.
