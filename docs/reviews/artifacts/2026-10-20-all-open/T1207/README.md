# T1207: empty candidates need no capture readiness

The permanent one-poll regression first fails: benchmark(empty) is Pending
because it waits for capture_ready. It now returns the original empty vector
before metadata cloning, readiness subscriptions or native work. The nonempty
path keeps its existing 15-second readiness contract. A permanent paused-clock
control requires that exact deadline, no selection and an empty result when
capture remains unavailable; the existing T627 matching-output/render trial
continues to pass.

All11 measured-trial cases pass normally and instrumented against the committed
base with pending T1199 source changes temporarily parked. No test deadline or
assertion was relaxed. Native LLVM line counters for benchmark are100%; source-
matched export includes only the actual current binary. Raw counters, command,
hash and red/green logs retained. Rustc1.98.1/cargo-llvm-cov0.8.7. Manual authored
source complexity estimates <=4 for the changed method and both new tests;
native Sonar measurement remains T1019. No other production function changed.

The function owns its vector for one invocation. Empty input transfers that
same zero-element allocation directly to the caller, retains no borrowed view,
creates no callback/task/native handle and requires no cleanup sponsor. Nonempty
input retains current bounded shortlist, capture/readiness and trial ownership.
No returned result grants permission or certifies a native resource as dead.
Existing held/cancellation/resource measurements remain in the normal suite.
This guard is not a physical rendering or unrestricted lifetime-safety claim.
No device/service, Windows/EVDI implementation or network action occurred.
