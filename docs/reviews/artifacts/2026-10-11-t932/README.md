# 2026-10-11-t932 — README

Original USB consent now joins application-owned isolated camera capture, with exact cleanup proof and display/camera capability enabled. Hosted tests do not establish physical USB or provider acceptance.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T932: original USB consent now composes with application-owned isolated camera capture. Process-root regression fails before wiring, then proves local Start/profile mapping and retained original Binder ownership: Stop/route close do not release the consumer until actual simulated Binder death/unlink. Source withdrawal regression is red before opaque wrapper-lifetime correction; equal metadata cannot revive an old acquisition claim. Default capability regression is red before changing display-only to display/camera (3); shared audio remains unavailable. Default production runner stages camera permission without opening an endpoint first. Portable tests cover all 128 foreground/background/permission/intent/admission/liveness combinations, exceptions, source completion/failure, parent cancellation, queued stale publications, no duplicate wrapper restart and actual proof forwarding. Existing private Binder/actor/real TCP joined regressions remain. Final normal suite: 1,782 tests, zero failed/skipped; all 69 affected and all 2,498 Android functions pass native coverage individually. Expanded whole-source complexity: 20,502 rows, zero above 9. Simulated API 27/34 and private fixtures are not physical USB/provider acceptance; T858/T863 remain pending and native trials are postponed. No downloads/device changes.

</details>
