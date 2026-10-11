package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

/** Application-owned camera composition on the camera dispatcher. Source loss
 * clears only its selected camera; actual native retirement remains in the lane. */
internal class CameraCaptureObservation(
    source: Flow<CameraCapturePublication?>,
    consent: StateFlow<MediaConsentView<CameraAccessOffer>>,
    permission: () -> Boolean,
    private val useTarget: (CameraCaptureTarget?) -> Unit,
    scope: CoroutineScope,
) {
    private var closed = false
    private val selection = CameraCaptureSelection({ consent.value }, permission)
    private val job = scope.launch {
        try {
            coroutineScope {
                val current = MutableStateFlow<CameraCapturePublication?>(null)
                launch {
                    try { source.collect { current.value = it } }
                    finally { current.value = null }
                }
                combine(current, consent) { value, _ -> value }.collect {
                    if (!closed) useTarget(selection.project(it))
                }
            }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { /* Native cleanup is still owned by CameraBinding. */ }
        finally { if (!closed) useTarget(selection.project(null)) }
    }

    fun retire() {
        closed = true
        job.cancel()
        useTarget(selection.project(null))
    }
}
