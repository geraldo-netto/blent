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

Retained endpoint cleanup retries after 100, 200, 400, 800, 1600 and 3200 ms,
then at five-second intervals, in addition to command execution time. Each new
cleanup owner starts at 100 ms. These intervals limit polling; they never release
an uncertain creator or removal. A recovered endpoint can therefore wait up to
five seconds for its next observation. Physical unplug/replug CPU and process
rates remain unmeasured; the cadence and ownership transitions have automated
virtual-time regression coverage.

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
the attempt. A successful, fully collected command carrying an exact socket
refusal for that endpoint also proves non-creation: `failed to connect to
'HOST:PORT': REASON` on POSIX or `cannot connect to HOST:PORT: REASON` on Windows.
The corresponding exact DNS failure forms (`failed to resolve host: 'HOST':
REASON` and `cannot resolve host 'HOST' and port PORT: REASON`) also prove that
creation did not begin. These replies originate before transport registration, so the attempt releases
its slot and address without polling inventory or issuing disconnect; a later
reconnect tick can try again. T1538 tests this refusal/retry/shutdown sequence.
The classifier rejects foreign endpoints, empty reasons, malformed UTF-8,
multiline replies and replies over 64 KiB.

Generic `failed to connect to HOST:PORT` and authentication failures may occur
after registration and remain uncertain. A non-zero command exit, deadline,
signal, oversized reply or unrecognized reply is also insufficient. Empty
inventories do not settle a possible delayed creator. This distinction follows
ADB's [connect workflow](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/client/transport_emulator.cpp)
and [POSIX socket failure replies](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/sysdeps/posix/network.cpp).
The owner waits to observe its exclusively owned endpoint, removes it once, then
waits for confirmed absence. Explicit Wi-Fi-off uses the same removal owner.
Wi-Fi-off first performs a fenced inventory read. A proven nonstart reports ADB
unavailable; a failed or malformed initial read reports an error. Neither creates
a cleanup obligation or repeats the failed read. A confirmed absent endpoint
finishes without disconnect. Once a disconnect starts, caller feedback is
separate from settlement: unknown outcomes retain the endpoint until absence is
confirmed. A proven disconnect nonstart returns its error and releases the off
attempt. Retained cleanup logs its first error once per episode.
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
