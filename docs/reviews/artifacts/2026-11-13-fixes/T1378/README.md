# T1378 — current identity authority

Both permanent regressions in `host/src/usb/tests/identity.rs` failed before the
fix and pass afterward. The full selected USB suite passed 24/24 cases.
`Network::observe` meets the per-function line gate (80%) and complexity 4.
Tools: rustc 1.98.1, cargo-llvm-cov 0.8.7, project syn 2.0.119 complexity frontend.

The monitor remains the owner of active identities and pending route debt.
Each observation pass clears only current alias authority before fallible probes;
failed/invalid observations cannot inherit the prior identity. Shutdown keeps
unconfirmed debt, rejects replacement identity, and retires it after the original
identity returns. The tests exercise the real portable monitor with synthetic ADB;
they do not establish native Windows or physical mapping acceptance. Existing
T1344–T1346/T1352 ownership blockers remain separate.
