# Project review follow-up — 2026-11-13

Reviewed revision: `5cfcfd8b`. This review records findings and evidence; production
code and permanent tests remain unchanged. The six new TODO rows are T1383–T1388.
The 15 existing blocked/declined rows retain their exact text and disposition.

| ID | Severity | Confirmed finding |
| --- | --- | --- |
| T1383 | medium | Linux lossily decodes invalid physical identities, allowing distinct invalid bytes to collapse two transport entries into one. Controls and oversized identities also differ from portable validation. |
| T1386 | medium | ADB inventory confirms serials that route/status contracts reject. That malformed observation can therefore be used as presence authority before route admission rejects it. |
| T1387 | low | Optional Wi-Fi reconnection delays healthy USB discovery and can consume the first whole-poll deadline. Later polls recover; permanent starvation was not reproduced. |
| T1384 | low | The Android Activity pairing test still expects the former nonremembering default. Its two SDK variants and three ordered-suite wrappers fail. |
| T1385 | low | Architecture, website and llms.txt authentication descriptions retain obsolete optional/tokenless application behavior; architecture also misstates token lifetime and delivery. |
| T1388 | low | The website advertises a current graphics-tablet overlay mismatch that current code and existing regression tests have corrected. |

Each row includes affected locations, evidence, required action and completion
conditions. T1387 retains one cohesive scheduling/settlement outcome; splitting
task admission from cleanup would leave its ownership requirement incomplete.
Documentation corrections require no artificial new tests. Behavioral changes
require permanent failing-before/passing-after regressions when implemented.

## Reproductions

The [artifacts](artifacts/2026-11-13-project-review-followup/) retain three local
Rust probes, their final output, source hashes and the probe dependency lock.

- T1383 executes the exact extracted `probe_device_identity` body using an owned
  local shell fixture. Responses `0xff` and `0xfe` become the same replacement
  character, and production transport selection returns only one of two supplied
  endpoints. The portable production adapter rejects both, an embedded NUL and
  a 1025-byte identity. This establishes input-validation divergence and grouping
  behavior; it does not establish a physical tablet substitution.
- T1386 calls the compiled production inventory parser and route constructor.
  An ESC-containing serial and a 1025-byte serial both produce confirmed inventory
  while route construction rejects them. The session-disappearance consequence
  follows the monitor's confirmed/unknown inventory branches. No hardware session
  was removed by this probe.
- T1387 executes the production Network implementation with its query owner and
  shared Wi-Fi attempt code. Its sole source adaptation is an explicit Rust module
  path for `queries`, retained in `network_body.inc`. Two four-second simulated
  Wi-Fi stages exhaust the monitor's six-second observation budget before any
  healthy USB identity query. Follow-up polls discover USB while original cleanup
  continues, and the process-exit fence waits until the simulated route is removed.

Probe resource ownership is bounded: temporary files belong to the probe process;
native shell command output uses the production child/collector adapter. The
inventory probe is stateless and retains no external resources. The reconnect
probe owns its fake state through Arc, stops query tasks, and fences original
Wi-Fi attempt retirement before exit. It issues no real ADB command or mapping
mutation. These observations do not prove unrestricted native memory safety.

## Review scope

This was a repository-wide pass with focused source inspection and automated
suites, not a claim that every source line received independent manual review.

| Domain | Inspected boundaries and evidence |
| --- | --- |
| Host discovery and lifecycle | Linux/portable inventory, identities, route admission, incremental tasks, Wi-Fi creation/cleanup, attachment credentials and native listener ownership. Existing display-route cleanup blockers remain separate. |
| Shared contracts | Transport grouping, serial validation, workspace name bounds, pairing authority, trust records, frame-window and credit contracts; compared matching Android representations. |
| Android | Token entry and preference observation, display-route switching, stream/activity coordination, remembered pairing and foreground ownership, settings, decoder configuration and connection framing. Full debug unit suite executed on hosted Android fixtures. |
| Desktop UI | USB device projection, connection controls, responsive columns, resource selection, platform composition and settings. Rust GUI suites and their normal smoke fixtures execute in the broad Rust run. |
| Native helpers | USB discovery/admission and frame-exchange allocation, publication and retirement paths; normal Rust integration suites exercise C helper and conversion fixtures. |
| Installation and distribution | Source/bundle staging, installed-file publication, autostart/service composition, release gates, APK checks and CI wiring; normal tooling regressions execute in the Rust run. |
| Documentation | README, current architecture/security/settings/compatibility/troubleshooting material, website and llms.txt; historical reports and explicitly declined work retain their dispositions. |

## Verification and limits

The Rust run passes 2,563 of 2,570 registered cases across 48 test executables;
seven existing opt-in benchmark cases are ignored and none fail. This includes
430 common-library cases and 2,133 host/GUI cases. The normal native helper,
conversion, installation/release and GUI smoke fixtures are included in those
parent test counts. Rust formatting passes. The supported-source complexity
check measures 28,928 functions, with none above nine.

The full Android run executes 2,718 registered cases across 366 reported suites:
2,713 pass, five fail, none are skipped. Both failing Activity cases report
`remember=false` expected versus `remember=true` actual. All three failing
SavedUiOrder wrappers include that same oracle. T1384 records one root cause.
Both API variants of the existing T241/T247 overlay regression pass, supporting
T1388 together with the current greeting and presentation code.

Final Rust test counts, commands, complexity and formatting results are in
[verification.json](artifacts/2026-11-13-project-review-followup/verification.json).
Nested Python/C executions are not added again to parent Rust test-case counts.
Complexity function counts describe automated measurement, not manually inspected
functions or implemented platform support.

No physical tablet, EVDI attachment, native Windows runner, reinstall, publication
or deployment was exercised. Optional PipeWire/libavcodec configurations and a
fresh whole-project per-function coverage collection were not run. Hosted Android
tests do not establish physical display streaming or resolve the earlier black
screen report. Existing native evidence and decision blockers remain in TODO.md.
