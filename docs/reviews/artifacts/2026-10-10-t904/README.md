# T904 — retire cleared Android session credentials

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

SessionCoordinator.applyToken previously returned immediately when the saved
ADB token became null, leaving old control credentials and active video retry
jobs intact. Four permanent API27/API34 regression cases failed before the fix.
Clearing a previously held credential now disconnects control, cancels video,
and clears both captured tokens. Late control callbacks cannot reopen that
retired video generation. Background clearing starts no work. Initial tokenless
ADB startup still permits its existing token-delivery handshake, and a later
new credential reconnects through the existing path.

SessionRevocationTest retains active/background, old-job cancellation, stale
callback, null-start, fresh replacement and lazy default-composition checks.
216 selected lifecycle/UI tests pass, none skipped. All 46 indexed functions
and all 103 executable native JVM methods in SessionCoordinator.kt meet 80%
individual line coverage, including primary constructors and getters. All 25
production functions have complexity at most 9. Raw counters, source manifest,
red/green logs and reports are retained. This resolves the stale ADB credential
bug extracted from T857; generation-scoped USB lease stop and actual display
transport integration still belong to T857. No device installation or download.
