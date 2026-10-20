package com.blent

import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine

/** Activity-scoped borrowed views of the three application media owners. Cold
 * observation belongs to WindowPowerBinding; no independent coroutine, flag,
 * native resource or consent authority is created here. */
internal class MediaScreenWakeSource(
    private val camera: StateFlow<CameraLifecycleState>,
    private val microphone: StateFlow<AudioLifecycleState>,
    private val speakers: StateFlow<AudioLifecycleState>,
) {
    val updates = combine(camera, microphone, speakers) { _, _, _ -> current() }

    fun current(): ScreenWakeDemand {
        val originalCamera = camera.value
        val originalMicrophone = microphone.value
        val originalSpeakers = speakers.value
        return ScreenWakeDemand {
            cameraNeeded(originalCamera) || audioNeeded(microphone, originalMicrophone) ||
                audioNeeded(speakers, originalSpeakers)
        }
    }

    private fun cameraNeeded(original: CameraLifecycleState): Boolean =
        camera.value === original && original.selected != null

    private fun audioNeeded(source: StateFlow<AudioLifecycleState>, original: AudioLifecycleState): Boolean =
        source.value === original && original.sharing
}
