package com.blent

import android.content.Context
import android.content.Intent
import android.os.Handler
import android.os.Looper
import com.blent.connection.*
import kotlinx.coroutines.*
import kotlinx.coroutines.android.asCoroutineDispatcher

internal object AudioOwner {
    private var microphone: AudioBinding? = null
    private var speakers: AudioBinding? = null
    private var control: AudioActivityOwner? = null
    private val prompts = LocalPermissionPrompts()
    private val observations = arrayOfNulls<AudioAccessObservation>(2)
    private var scope: CoroutineScope? = null
    fun access(direction: Int): MediaAccessControls<AudioAccessOffer>? {
        cached(direction) // Validate before indexing.
        return observations[direction - 1]?.policy
    }
    private fun cached(direction: Int): AudioBinding? = when (direction) {
        1 -> microphone
        2 -> speakers
        else -> error("Invalid audio direction.")
    }
    fun get(context: Context, direction: Int = 1): AudioBinding {
        cached(direction)?.let { return it }
        val app = context.applicationContext
        val service = if (direction == 1) MicrophoneService::class.java else SpeakerService::class.java
        val binding = RemoteMediaBindings.audio(app, direction, { requestPermission() }, service = { run ->
            val intent = Intent(app, service).putExtra("audio_run", run)
            if (run == null) app.stopService(intent) else app.startForegroundService(intent)
        })
        if (direction == 1) microphone = binding else speakers = binding
        val dispatcher = scope ?: CoroutineScope(SupervisorJob() + Handler(Looper.getMainLooper()).asCoroutineDispatcher()).also { scope = it }
        val resource = if (direction == 1) ConnectionManagement.Resource.MICROPHONE else ConnectionManagement.Resource.SPEAKERS
        observations[direction - 1] = AudioAccessObservation(binding, ConnectionAudioAccess(ConnectionOwner.get(app), resource), prompts, dispatcher)
        return binding
    }
    fun attach(launch: (LocalPermissionPrompt) -> Boolean): LocalPermissionAttachment {
        val owner = control ?: AudioActivityOwner(checkNotNull(microphone) { "Microphone owner not initialized" },
            checkNotNull(speakers) { "Speaker owner not initialized" }, prompts,
            checkNotNull(observations[0]).policy, checkNotNull(observations[1]).policy).also { control = it }
        return owner.attach(launch)
    }
    fun detach(identity: LocalPermissionAttachment): Boolean = control?.detach(identity) == true
    fun permissionResult(prompt: LocalPermissionPrompt, granted: Boolean): Boolean = prompts.result(prompt, granted)
    private fun requestPermission() {
        if (control?.requestPermission() == true) return
        microphone?.let { it.permissionResult(false, it.permissionPending) }
    }
    fun owns(run: String?, direction: Int = 1): Boolean = cached(direction)?.ownsBackground(run) == true
    fun stopped(run: String?, direction: Int = 1) { cached(direction)?.backgroundStopped(run) }
    fun stopSharing(direction: Int = 1) { cached(direction)?.stopSharing() }
    fun reset() {
        observations.forEachIndexed { index, value -> value?.retire(); observations[index] = null }
        scope?.cancel(); scope = null
        val owner = control
        control = null
        if (owner != null) owner.clear() else { microphone?.shutdown(); speakers?.shutdown() }
        prompts.clear(); microphone = null; speakers = null
        AudioInvitations.microphone.value = null; AudioInvitations.speakers.value = null
    }
}
