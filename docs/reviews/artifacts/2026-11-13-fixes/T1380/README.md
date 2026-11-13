# T1380 — revalidate Linux endpoint identity

Both permanent `monitor::identity_tests::t1380_*` cases fail before the fix and
pass afterward. The selected Linux monitor suite passed 55/55 cases. All three
changed production functions exceed 80% line coverage and score at most 6.
Tools: rustc 1.98.1, cargo-llvm-cov 0.8.7, project syn 2.0.119 complexity frontend.

Each scheduled observation queries current identity. Discovery no longer keeps
an unused second identity copy; Monitor owns the published identity. Existing
per-endpoint tasks, observation epochs, cancellation and native command cleanup
remain authoritative. Same-endpoint replacement revokes the old attachment;
failed revalidation publishes unknown, and a later observation recovers. The
unrelated tablet's identity and consumed launch ticket remain intact.
Native ADB is represented by owned local shell fixtures; no tablet was mutated.
