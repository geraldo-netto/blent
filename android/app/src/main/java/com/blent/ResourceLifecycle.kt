package com.blent

/** A run retains ownership until execute and retire finish. Cancellation only
 * requests Stop; it never proves native retirement or authorizes replacement. */
internal interface LifecycleRun {
    suspend fun execute()
    fun cancel()
    suspend fun retire()
}

internal data class AudioLifecycleState(
    val preferences: AudioPreferences,
    val status: String,
    val sharing: Boolean = false,
    val pending: AudioEndpoint? = null,
)

internal data class CameraLifecycleState(
    val endpoint: CameraEndpoint? = null,
    val selected: CameraLens? = null,
    val status: String = "Start camera sharing on your computer.",
)
