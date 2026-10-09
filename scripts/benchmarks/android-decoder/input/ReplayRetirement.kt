package com.blent.benchmark

/** Replay workers may await native retirement; Activity/UI callbacks must not. */
internal fun awaitReplayRetirement(
    active: () -> Boolean,
    pending: () -> Boolean,
    now: () -> Long = System::nanoTime,
    pause: () -> Unit = { Thread.sleep(1) },
) {
    val started = now()
    while (pending()) {
        check(active()) { "Replay stopped during native retirement" }
        check(now() - started < 5_000_000_000L) { "Replay native retirement stalled" }
        pause()
    }
}
