package com.blent

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import org.json.JSONObject

/** One request job owns this retry loop. Native queries retain their own exact
 * retirement; neither an unavailable reply nor cancellation releases that lane. */
internal suspend fun awaitCapabilities(
    query: suspend () -> JSONObject,
    wait: suspend (Long) -> Unit = { delay(it) },
): JSONObject {
    var backoff = 100L
    while (true) {
        currentCoroutineContext().ensureActive()
        val report = availableCapabilities(query)
        if (report != null) return report
        wait(backoff)
        backoff = (backoff * 2).coerceAtMost(2_000L)
    }
}

private suspend fun availableCapabilities(query: suspend () -> JSONObject): JSONObject? = try {
    query().takeUnless { it.has("status") }
} catch (cancelled: CancellationException) {
    throw cancelled
} catch (_: Exception) {
    null
}
