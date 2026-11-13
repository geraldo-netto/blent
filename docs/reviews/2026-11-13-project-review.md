# Project review — 2026-11-13

Reviewed revision: `8a471d0a` (full hash and finding-source hashes in the
[source manifest](artifacts/2026-11-13-project-review/source-manifest.json)).
This pass changes the TODO ledger and review evidence only. It does not fix
production code, add permanent regressions, commit, reinstall or touch physical
ADB mappings.

Four confirmed findings are recorded in [TODO.md](../../TODO.md):

| Finding | Severity | Confirmed behavior |
| --- | --- | --- |
| T1378 | medium | Portable discovery retains old identity after a failed current probe; that cache authorizes route retargeting/retirement. |
| T1380 | medium | Linux discovery never re-queries a known identity while its endpoint remains listed, missing a replacement tablet at that endpoint. |
| T1381 | medium | Sequential identity probes exceed the portable monitor's whole-inventory deadline repeatedly, starving a healthy device behind slow devices. |
| T1379 | low | A Linux CLI compatibility re-export remains despite the project's no-backward-compatibility policy. |

These are separate outcomes: T1378 addresses failed observations in the portable
monitor; T1380 addresses unconditional caching in Linux discovery; T1381 addresses
independent discovery progress. T1379 is a structural policy contradiction.
The 14 existing Other-item rows are unchanged.

## Evidence boundaries

The three Rust probes include the reviewed production source files directly and
inject synthetic ADB/identity observations. Their sources and compiler/output
logs are retained in the [artifact directory](artifacts/2026-11-13-project-review).
They do not issue physical ADB commands or modify production source files.
They are review reproductions, not permanent automated regressions; the TODO
rows specify the required fail-before/pass-after coverage for implementation.

T1378 observes `None` from a fresh identity query, the old cached identity and
`can_retire=true`. It proves stale cleanup authorization, not an observed physical
mapping deletion. T1380 completes two inventory observations across an injected
identity change and counts only one identity query. T1381 supplies two four-second
identity replies before an immediately responsive device and applies the same
six-second deadline used by the portable monitor; both polls time out before the
healthy device is queried. Its continued starvation is conditional on that
unchanged ordering and delay pattern.

The probe adapters and temporary configuration directories are process-owned;
all probe calls complete or are cancelled by the existing protocol deadline,
and temporary directories are released on return. They acquire no external
native device ownership. Production cleanup debt must remain retained when
fixing the identity findings; dropping unknown ownership is not a resolution.

## Review scope

This was a cross-component source and regression pass, not an assertion that
every repository line or possible interleaving was exhaustively verified.

- Host: Linux and portable discovery, attachment identity, route retargeting,
  Wi-Fi ownership/exit, input dispatch, media registration/storage and capture
  selection/cache boundaries.
- Shared contracts: configuration merging/persistence, platform capabilities,
  authentication/trust, resource transitions, multiplexing and raw-frame bounds.
- Android: token entry points, control/video framing and decoder setup,
  configured connections, camera/audio consent and resource retirement,
  preferences, update checks and manifest boundaries.
- GUI: device naming and responsive presentation, resource selection/ownership,
  settings transactions, status polling and installed notices.
- Native C: USB selection/debugging reservations, transfer ownership and raw-ring
  allocation, descriptor handling, publication and retirement.
- Distribution/tooling: source/AppImage publication, shell renderers, Arch and
  portable builds, release verification, CI boundaries and current installation/
  Windows documentation. Historical Debian fixtures and dated acceptance reports
  were distinguished from active distribution behavior.

No further defect was confirmed in those sampled paths. Passing tests and source
analysis do not establish unrestricted memory safety, native Windows support,
physical tablet acceptance or fresh resolution of the earlier Android black-screen
report. The existing TODO blockers retain those limits. Optional PipeWire and
libavcodec configurations were not built in this pass.

## Verification

Fresh validation completed:

| Check | Result |
| --- | --- |
| Rust common, host and GUI cases | 2,552 of 2,559 registered cases passed; seven existing opt-in benchmark cases ignored; zero failures |
| Selected Android unit cases | 76 of 76 passed across seven selected suites |
| Selected USB-helper script cases | 11 of 11 passed |
| Selected AppImage script cases | 32 of 32 passed |
| Supported-source complexity | 28,893 of 28,893 measured functions scored at most 9 |
| Rust formatting and diff whitespace | Passed |

Rust integration totals include the 70-case EVDI helper suite, the 12-case
conversion suite, installation/release tooling and GUI startup/accessibility
smoke tests. Embedded Python/C fixture executions are not added again to those
Rust test-case counts. Native device simulation does not certify physical devices.

Final commands, test counts, exclusions, tool identities and retained logs are
listed in [verification.json](artifacts/2026-11-13-project-review/verification.json).
The complexity checker measures its supported source scope, including tests and
review probe sources; its function count is not a count of functions manually
reviewed or proof of per-function executable coverage. No new coverage claim is
made by this review.
