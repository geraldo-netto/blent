# T1057: original Android display eligibility

The original control inbox validates supported/unsupported/unavailable observation
states, nonregressing counters, exact duplicates and remembered stop fences.
Current malformed observations close that original control owner. Immutable views
carry an opaque original identity independent of wire counters; late views cannot
authorize another connection or foreground epoch. The coordinator requires the
exact current authenticated observation, current mode, supported format and codec.
It retains configuration while withdrawing decoder demand on pause or source loss.

The existing ResourceLane owns actual decoder retirement and replacement. Joined
SDK 27/34 tests hold its original retirement open through 32 coalesced interruptions:
old render receipts become invalid, late callbacks have no effect, and replacement
cannot start until that original retirement settles. Background, replacement,
malformed source, legacy fallback, configuration and rejected-settings paths remain
covered. Fresh workers retain selected routes/format and use the existing fresh
video connection; actual fresh header/IDR production is independently retained in
T1016/T1009 host fixtures. These tests do not establish hardware decoding or physical
presentation. T1011 retains final joined Android acceptance; T914 retains physical
host/tablet acceptance and T1058 wake-resource integration.

The UI bridge retains one admitted drain and one latest view across foreground
epochs. A per-continuation guard accounts for rejected, enqueue-then-throw and
duplicate scheduling. Cancellation cleanup runs without applying a native/UI update
from a fallback thread. Error reporting cannot retain an original admission or
mutate Activity state off its owner thread. Permanent dispatch tests coalesce 1,000
views, replace epochs and exercise reporter faults. Inbox tests bound 84 numeric
transition combinations per SDK. The independent lifetime, ownership, conservation,
cutoff and conditional eventual-cleanup assumptions are recorded in the
[signal contract](../../../display-activity-signal.md).

Permanent regressions were observed failing before their fixes: inactive greeting
admission and MIME overriding unsupported format (both SDKs), rejected dispatcher
admission cleanup, rejected settings restarting ahead of a queued pause (both SDKs),
and off-thread Activity error mutation (both SDKs). Raw red logs and faulty controls
are retained. Existing route/reconnect fixtures now use actual current greeting and
connection setup rather than bypassing original ownership; all prior authentication,
stale callback and route assertions remain. The preliminary full run's two fixture
failures and diagnostic coverage gaps are retained as nonpassing evidence.

Final offline ordinary suite: **2,033 tests, zero failures/errors/skips**. Native
JaCoCo 0.8.15 measures all 64 maintained authored production methods independently
at **87.5% or higher**. The pinned Kotlin PSI 1.9.20 inventory uses the existing
compiler-generated/inline attribution policy with no new exclusions. Complete
source/vector hashes, relevant class hashes, raw execution/XML counters and method
inventory are retained. A cached source complexity audit covers 507 production/test
functions, maximum 8; this is an estimate, not SonarQube certification (T1019).
Twelve architecture/contract/portability checks pass. No network transfer, package
installation, push, live suspend/lock or tablet interruption occurred.
