# T950: original legacy camera permission identity

Three permanent regressions failed before the identity guard, then passed in
the complete camera/connection/lifecycle fixture suite. A stale, forged, duplicate
or stopped SDK result cannot consume the newer request. Repeating the same
endpoint/lens still creates a distinct request. Fresh permission is checked again
before capture; a reported grant without permission cannot start a worker.

All 32 inventoried production functions pass individual native line counters.
Authored constructors are additionally retained until T848 repairs the normal
inventory. The default result argument preserves existing internal callers;
T944 must pass the captured original request from the Activity SDK adapter.
This commit does not wire USB consent or claim physical camera acceptance.
