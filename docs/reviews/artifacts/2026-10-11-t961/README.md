# T961: original camera Activity/SDK ownership

Permanent native Activity regression reproduces an older stop shutting down the
replacement's process camera binding. Portable regressions reproduce replacing
an outstanding SDK request and consuming a newly launched request during attach.
All same regressions pass after the owner/launcher identity guards.

Application owner attaches exactly one launcher; old stop/destroy cannot clear or
stop its replacement. Legacy permission actions retain the original request and
application state, never an Activity. Activity SDK slot keeps its original prompt
until actual result/destruction, rejects overlap, and clears before dispatch for
synchronous/reentrant completion. Original/stale/double/denied/reset/failed callbacks
cannot grant a replacement selection. Background service behavior is retained.

All 93 production functions pass native individual line counters; authored
constructors are additionally supplemented until T848. All reviewed production
and regression functions stay <=9. Full camera/connection/Activity/lifecycle suite
passes without skips on simulated SDK 27 and 34 where applicable. No physical
SDK/provider or camera acceptance is claimed; device testing remains postponed.

USB offer/process observation remains T944, actual camera composition T932 and
physical camera acceptance T858. The implemented capability mask remains display
only. No downloads, install/reload, EVDI modifications or live device disruption.
