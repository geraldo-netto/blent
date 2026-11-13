# Lifecycle review and fixes — 2026-11-13

Resolved 22/22 confirmed findings (T1427–T1448) in 22 separate local commits:
19 behavioral fixes and three documentation/maintenance items. Resolved TODO
rows were removed; the original nine blocked and eight declined rows remain exact.

Baseline: `31912bc7`. This round followed the earlier broad project reviews,
then traced Android connection transitions, Linux activity recovery, daemon and
GUI shutdown, native input ownership, mapping and OSK command settlement.
Three reviewers worked by area; the parent checked cross-domain interactions.

Confirmed findings were entered in `TODO.md` before fixes. Each behavioral fix
retains a permanent automated regression demonstrated failing before and passing
after the change. T1437 corrects documentation; T1447 updates deprecated atomic
API calls and adds nonce-exhaustion coverage while preserving behavior; T1448
restores the CI formatting gate. Each resolved finding has its own local commit.
Original blocked and declined dispositions remain unchanged.

| Finding | Corrected behavior | Evidence |
| --- | --- | --- |
| T1427 | Retry transient activity refresh failures | [Report](T1427.md) |
| T1428 | Remove owned PID publication after failed startup | [Report](T1428.md) |
| T1429 | Retry failed initial activity-property queries | [Report](T1429.md) |
| T1430 | Reject Linux input instances outside the funded identity range | [Report](../2026-11-13-lifecycle-fixes/T1430.md) |
| T1431 | Allow Android Disconnect during remembered USB retry waits | [Report](android-transitions/T1431.md) |
| T1432 | Retain control workers through process shutdown | [Report](T1432.md) |
| T1433 | Retain original OSK operations and serialization through cancellation | [Report](T1433.md) |
| T1434 | Retain failed touch-release obligations before accepting fresh contacts | [Report](T1434.md) |
| T1435 | Retain Linux GUI cleanup through process exit | [Report](T1435.md) |
| T1436 | Retry initial display-state failures | [Report](T1436.md) |
| T1437 | Document configuration-worker capacity and tradeoffs | Commit `a2b90974`, `docs/architecture.md` |
| T1438 | Retry transient session-identity acquisition failures | [Report](T1438.md) |
| T1439 | Retain input payload until original mapping and cleanup settle | [Report](T1439.md) |
| T1440 | Retain interrupted CLI Wi-Fi setup and cleanup | [Report](T1440.md) |
| T1441 | Sponsor direct OSK restoration before process cutoff | [Report](T1441.md) |
| T1442 | Admit activity capabilities only after valid initial evidence | [Report](T1442.md) |
| T1443 | Publish complete daemon PID atomically | [Report](T1443.md) |
| T1444 | Retain delegated mutation acknowledgment independently of caller waits | [Report](T1444.md), [common execution](T1444-common.md), [adapters](T1444-adapters.md) |
| T1445 | Match X11 mapping to the actual device creation | [Report](T1445.md) |
| T1446 | Address X11 mutation/readback by creation name across helper connections | [Report](T1446.md) |
| T1447 | Replace deprecated atomic calls and verify nonce exhaustion | [Report](T1447.md) |
| T1448 | Restore the repository Rust-format gate after review changes | [Report](T1448.md) |

Per-item artifacts identify exact source snapshots, regression commands/results,
coverage counters, complexity measurements and limits. They describe each fix
at its own verification snapshot; later dependent fixes are listed above. Counts
describe selected executions or explicitly inventoried functions; overlapping cohorts are not
summed into a whole-project coverage or compatibility claim.

Private D-Bus, process namespaces, fake native input, hosted Android and private
Xvfb fixtures exercised failure, cancellation, delayed completion and reuse.
The final maintenance checks also exercised the optional encoder in software
using the installed SDK; per-function results are recorded in T1447.
No tablet configuration, installed application, running user service or physical
display/input setup was changed. Windows cross-compilation is compilation
only; the existing native Windows acceptance blockers remain. Declined macOS,
stylus and performance campaigns were not resumed.

A native mutation with a lost/ambiguous acknowledgment remains owned and funded;
its process exit or elapsed caller deadline cannot establish remote settlement.
Cleanup may therefore remain pending indefinitely when the original outcome is
unknown. Eventual cleanup assumes original acknowledgment and native settlement.

This is a finite, evidence-backed review of the stated paths and their adjacent
failure cases. It does not prove that the repository contains no other defects.
