# T1043: shared activity work observation and original watch view

The independent `WorkObservation` value contract binds one resource target to its
original activity stamp. Pending/failed/stopped publications, invalid snapshots,
supported-source loss/closure and subsequent stop fences deny or retire desired
work. Coalesced pause/resume cannot revive an original observation. Foreign/future
stamps fail closed; display observations cannot be reused as auxiliary observations.
Lock/display demand leaves auxiliary work eligible; machine suspension stops both.

Explicitly absent activity composition and known unsupported target capabilities
retain the established fallback, including source closure/loss. Unsupported does
not override actual no-viewer demand. Moving between unsupported and supported
observation invalidates prior work. Display observes power/lock/display capability;
auxiliary observes power capability. No token grants native acquisition, current
attachment authority or actual retirement proof.

The host `activity_view::View` adapts the original read-only watch to that portable
contract. Notices arm synchronously before the caller reads current state; a
publication or closure before first poll still wakes. Already closed/absent sources
wait without a polling loop. Replacements retain separate source views. The
adapter duplicates neither policy nor mutable publication state.

Five initial independent assertions fail with permissive draft observation and
pass unchanged with strict interpretation. Permanent normal-suite cases cover
bounded invalid epochs/revisions/fences, foreign and future publications, invalid
demand consistency, coalesced lock/suspend transitions, target mismatch, all eight
capability masks under open/closed/lost states, unsupported fallback, no-viewer
separation, arm-before-first-poll source closure and original/replacement isolation.

Fresh offline suites pass 797 host-library and 350 shared-contract tests, with
three existing benchmark ignores. Seven architecture/portability checks pass.
All eleven maintained production functions measure 100% executable-line coverage
from native LLVM counters, with source/native-object attestations and raw LCOV.
Source audit: 25 production/test functions, max complexity 9, unchanged repository
Sonar-rule counting with cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2. Native
SonarQube remains unavailable (T1019), and source scores are not certification.
No download, install, push, physical power transition or native media acquisition
occurred. Selector, capture and auxiliary policy consumption remain T1037, T1015
and T1010; observation readiness is not their runtime acceptance.
