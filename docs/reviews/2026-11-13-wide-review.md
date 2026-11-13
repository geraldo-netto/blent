# Project review — 2026-11-13, wide follow-up

Reviewed revision: `04d77f88eaae732e925f254c817b66baefb940d4`.
Finding-source hashes and retained output are in the
[evidence directory](artifacts/2026-11-13-wide-review/).

Six findings were added to [TODO.md](../../TODO.md). Owner decisions during
the review declined three of them and changed the ledger to domain tables.

| ID | Severity | Status | Finding |
| --- | --- | --- | --- |
| T1396 | medium | open | Android recreation replays a consumed USB launch Intent and reconnects after explicit Disconnect. |
| T1395 | low | open | The media-retirement regression races Tokio virtual time against a real cleanup thread. |
| T1397 | low | wont_fix | JavaScript functions are omitted from complexity measurement; owner declined implementation. |
| T1398 | low | wont_fix | PowerShell functions are omitted from complexity measurement; owner declined implementation. |
| T1399 | low | wont_fix | Production Cinnamon JavaScript is omitted from per-function line coverage; owner declined implementation. |
| T1400 | low | open | The required Rust formatting check rejects the T1383 regression in `linux_main.rs`. |

All 16 preexisting ledger items retain their descriptions and dispositions.
All 22 current items appear once across seven domains. Project instructions and
persistent Blent preferences now require one table per domain, with lifecycle
in the status column. Production behavior and normal test sources are unchanged;
no commit or push was made.

## Findings and evidence boundaries

T1396 follows the actual `MainActivity.onCreate` attachment dispatch into
`ConfiguredConnection`: after a settled Disconnect, the restored launch Intent
is admitted as a new attachment. The hosted reproduction produced the same
incorrect second worker on API27 and API34. Four control cases passed: ordinary
launcher restoration preserves Disconnect, while a fresh attachment starts a
new worker. This does not establish physical USB behavior or bypass of fresh
TLS proof. Deny/Revoke recreation coverage is required with the future fix;
those variants were not separately reproduced here.

T1395 failed in the initial workspace run and once in 20 isolated executions
of the unchanged regression. The paused Tokio clock can advance its timeout
before the OS reaper runs. This is evidence of a test scheduling defect, not
production cleanup taking a real second or a demonstrated leak.

T1397–T1399 are measured inventory omissions, not assertions that an omitted
function exceeds complexity 9 or has less than 80% coverage. Their declined
status does not turn missing measurements into passing results. T1400 is a
formatter mismatch, with no behavioral defect asserted.

## Scope and limits

The review covered source and existing checks across host discovery and
connections, shared authentication/configuration and retirement, Android
connection/media lifecycle, GUI status/settings, native capture/input helpers,
installation/packaging, verification tooling and active documentation. Existing
findings and prior reports were checked to avoid reopening declined work or
duplicating known native-validation blockers.

This is a cross-component review, not exhaustive manual inspection of every
tracked line or proof of unrestricted safety. No fresh native Windows, physical
tablet, live Cinnamon or EVDI display acceptance is claimed. Existing ownership
decision blockers and native-validation requirements remain in the ledger.

## Verification already obtained

The owner instructed this review to disregard tests while the Rust continuation
was running. That owned process group was interrupted and confirmed empty;
the continuation is incomplete, not a passing workspace run. No further test
execution followed that instruction. Earlier evidence is retained, not discarded.

Completed beforehand: Android's 2,740 registered unit cases passed; the script
suite passed 565 of 566 registered cases with one Windows-only skip; the
complexity and coverage checker suites passed 36 of 36 and 111 of 111 cases.
The selected all-feature host run passed 1,505 of 1,510 registered cases with
five existing ignored cases across six Rust executables. These runs overlap
and their counts must not be summed as distinct coverage.

The final supported-language complexity inventory measured 29,058 functions,
all at most 9, including review evidence sources. This is neither a manual
review count nor executable-line coverage evidence; the language omissions
remain explicit. Local path checks across 33 active Markdown documents found
no missing local targets; external URLs and anchors were not verified.

Commands, tool versions, per-program summaries and limitations are recorded in
[verification.json](artifacts/2026-11-13-wide-review/verification.json).
