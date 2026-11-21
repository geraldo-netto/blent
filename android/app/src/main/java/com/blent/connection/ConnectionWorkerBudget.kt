package com.blent.connection

import java.io.IOException
import kotlinx.coroutines.withTimeoutOrNull

/** Bounds one coroutine wait, never the lifetime of the native operation it
 * observes. Only this budget's expiry becomes a failure; caller cancellation
 * and nested deadlines propagate with their original identity. */
internal suspend fun <T : Any> connectionWorkerBudget(stage: String, millis: Long, work: suspend () -> T): T {
    require(millis in 1..60000)
    return withTimeoutOrNull(millis) { work() }
        ?: throw IOException("Connection worker $stage timed out after $millis ms")
}
