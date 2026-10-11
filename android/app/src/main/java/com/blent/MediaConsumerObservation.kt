package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

/** Application-owned media composition on its dispatcher. Source loss
 * clears only its selected media; actual native retirement remains in the lane. */
internal class MediaConsumerObservation<O : MediaAccessOffer, E, T : MediaConsumerTarget<E>>(
    source: Flow<MediaConsumerPublication<T>?>,
    consent: StateFlow<MediaConsentView<O>>,
    permission: () -> Boolean,
    private val useTarget: (T?) -> Unit,
    scope: CoroutineScope,
    target: (E, MediaConsumerAuthority) -> T,
) {
    private var closed = false
    private val selection = MediaConsumerSelection({ consent.value }, permission, target)
    private val job = scope.launch {
        try {
            coroutineScope {
                val current = MutableStateFlow<MediaConsumerPublication<T>?>(null)
                launch {
                    try { source.collect { current.value = it } }
                    finally { current.value = null }
                }
                combine(current, consent) { value, _ -> value }.collect {
                    if (!closed) useTarget(selection.project(it))
                }
            }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { /* Native cleanup is still owned by the binding. */ }
        finally { if (!closed) useTarget(selection.project(null)) }
    }

    fun retire() {
        closed = true
        job.cancel()
        useTarget(selection.project(null))
    }
}
