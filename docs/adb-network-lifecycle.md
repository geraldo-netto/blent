# Network ADB endpoint ownership

Owner decision (2026-11-12): Blent exclusively manages an endpoint it connects
until creation and required cleanup are confirmed. Other programs may use ADB,
but must not connect or disconnect that endpoint, including aliases, during this
ownership. Unrelated transports remain outside Blent's ownership.

Each attempt owns one bounded retirement slot, an address reservation and a
native cross-process stripe lock. Reservations, locks and observations use the
same canonical address spelling. DNS aliases require the same external exclusivity;
Blent does not claim to identify all names for a remote device.

Cancellation after an acknowledged new connection requires compensation. The
owner joins each original command's child and output collector before observing
its effect. Failed disconnects retain the slot and lease. An unacknowledged
removal is issued only once: a later valid inventory without the endpoint settles
that original removal. Offline, connecting and unauthorized transports still
count as present; malformed or failed inventories never prove absence.

The caller's six-second wait bounds response latency, not cleanup lifetime.
Cleanup continues in its admitted worker after caller loss. A panicked worker
retains its slot and native lease. Unknown outcomes consume capacity until exact
settlement; successful observations and native retirement are conditional liveness
requirements, not guaranteed by a timer. `BLENT_WIFI_ATTEMPTS` bounds these owners
(default 8, configurable 1–64). Fixed native lock stripes can reject unrelated
addresses on collision rather than allow overlapping ownership.

The common command adapter certifies pre-spawn rejection on Linux. On other
platforms, an error without the child proof leaves the attempt retained, because
platform job setup could have spawned an unreported child. Linux tests exercise
native child completion and pure/fake route transitions. Windows native completion
and counters remain subject to the existing platform validation ledger items.

T1363 regressions cover failed rollback, delayed removal observation, caller loss,
replacement exclusion, unrelated admissions, alias reservations, malformed and
nonready inventories, safe pre-spawn retry and native command completion.

Before connecting, a valid inventory distinguishes a preexisting endpoint from a
new creator. Preexisting transports are preserved without issuing connect or
compensating disconnect. Otherwise the owner marks creation uncertain before
calling the native adapter. A spawn rejection proved by that adapter can release
the attempt; errors, oversized replies and unrecognized replies after possible
creation cannot. Empty inventories do not settle a possible delayed creator.
The owner waits to observe its exclusively owned endpoint, removes it once, then
waits for confirmed absence. Explicit Wi-Fi-off uses the same removal owner.
T1364 tests exercise effects both before and after acknowledgement failure,
preexisting transports, malformed snapshots, bounded replies and proven nonstarts.

Graceful host process exit closes Wi-Fi admission and waits for every admitted
attempt to settle after its producers stop. An unknown outcome can therefore
keep the process alive after its foreground work returns. The process must remain
alive for this guarantee: forced OS termination or power loss cannot run the
shutdown fence. Blent does not claim durable recovery across those events.
Successful retained connections transfer to configured reconnection policy;
they have no pending compensating cleanup when their attempt finishes.

T1365 checks graceful process-entry teardown with a delayed removal, preserves
the original return value, and checks that cutoff rejects new creators before
waiting for occupied slots. The GUI does not directly create network ADB attempts;
the host process owns this fence on both platform entry paths.
