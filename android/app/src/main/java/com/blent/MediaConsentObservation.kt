package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.collect

/** Process-owned portable observation, independent from Activity and transport.
 * Source completion/failure revokes consent; UI retirement cancels observation
 * only, never substitutes for native media/endpoint cleanup. Serialized with
 * the policy on its media dispatcher. */
internal class MediaConsentObservation<O : MediaAccessOffer>(
    source: Flow<O?>,
    private val policy: MediaConsentPolicy<O>,
    scope: CoroutineScope,
) {
    private var closed = false
    private val job = scope.launch {
        try { source.collect { if (!closed) policy.observe(it) } }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { /* Only this media source becomes unavailable. */ }
        finally { if (!closed) policy.clear() }
    }

    fun retire() {
        closed = true
        job.cancel()
        policy.clear()
    }
}
