package com.blent.connection

import kotlinx.coroutines.channels.ReceiveChannel
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.selects.onTimeout
import kotlinx.coroutines.selects.select

/** Coroutine adapter for ConnectionWait's 0..1000 ms policy. Borrows the actor's
 * conflated hint channel only until this wait settles; no native retirement is
 * implied. A due (zero) wait never consumes a retained hint. */
@OptIn(ExperimentalCoroutinesApi::class)
internal suspend fun awaitConnectionWake(wake: ReceiveChannel<Unit>, delayMs: Long) {
    require(delayMs in 0..1000)
    if (delayMs == 0L) return
    select<Unit> {
        wake.onReceive { }
        onTimeout(delayMs) { }
    }
}
