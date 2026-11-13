# Lifecycle recheck — 2026-11-13

Reviewed `9f8660aa`, after the six T1411–T1416 fixes. Three delegated reviewers
rechecked the requested areas; the parent cross-checked findings and examined
Linux input retirement at the session boundary. Seven confirmed findings were
recorded in the root TODO ledger, preserving its 17 existing dispositions.

| Scope | Findings | Evidence |
| --- | --- | --- |
| Service shutdown and startup | T1417 obsolete deadline contract/regression; T1421 activity-source exit ownership; T1423 startup SIGTERM window | [Service review](service/review.md) |
| Linux activity recovery | T1418 delayed owner return disables observation | [Linux review](linux/review.md) |
| Android lifecycle | T1419 background delivery ordering; T1420 null-choice cutoff suppression | [Android review](android/review.md) |
| Parent session/input cross-check | T1422 native uinput creation outlives canceled follower | [Input review](input/review.md) |

These are source traces and isolated reproductions. Existing regressions can pass
while the newly demonstrated ordering/failure cases remain uncovered. One existing
shared command regression fails against the changed service policy; the review
does not reinterpret that failure as an incorrect infinite cleanup requirement.

No production code, normal-suite tests, installed applications, running Blent
service or tablet settings were changed. The startup signal probe used private
namespaces, masked device nodes and fake ADB; input construction used the existing
fail-closed syscall shim. No physical suspend, native Android lifecycle or
Windows acceptance is claimed. Probe source/logs are review evidence, not
permanent regressions; each TODO row specifies required fix acceptance.
