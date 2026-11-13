# T1394 — release the synthetic creator on fixture loss

The permanent fixture-drop regression fails without the release sponsor and
passes with it; all three reconnect-fixture tests pass. A fixture-owned Drop
notification releases the synthetic creator even on test failure/unwind. Existing
Attempt ownership still performs original cleanup and observes native settlement.
The test always releases its faulty control before asserting failure, so the
negative demonstration also settles. Its two-second observation budget accommodates
the asynchronous retirement collector; the initial 100ms probe could race that
collector and was not a valid completion budget.

The pure fixture Drop retains one Arc only until teardown. Notify coalesces the
release permit; native operation and address settlement remain with Attempt.
No production code or admission policy changes. The scoped complexity check
measures ten functions, maximum 5, with the project syn 2.0.119 frontend.
