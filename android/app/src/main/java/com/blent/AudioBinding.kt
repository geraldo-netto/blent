package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/** Portable consent/admission/retirement policy. Platform adapters supply runs. */
internal class AudioBinding(
    initial: AudioPreferences,
    private val requestPermission: () -> Unit,
    private val permission: () -> Boolean,
    private val createRun: (AudioEndpoint, AudioPreferences, (String) -> Unit) -> LifecycleRun,
    private val invitations: MutableStateFlow<AudioEndpoint?>,
    private val scope: CoroutineScope,
    private val service: (String?) -> Unit,
    val direction: Int,
    private val save: (AudioPreferences) -> Unit,
) {
    val label = if (direction == 1) "microphone" else "speakers"
    private val mutableState = MutableStateFlow(AudioLifecycleState(initial, "Start $label on your computer."))
    val state = mutableState.asStateFlow()
    var preferences: AudioPreferences
        get() = mutableState.value.preferences
        private set(value) { mutableState.update { it.copy(preferences = value) } }
    var status: String
        get() = mutableState.value.status
        private set(value) { mutableState.update { it.copy(status = value) } }
    var sharing: Boolean
        get() = mutableState.value.sharing
        private set(value) { mutableState.update { it.copy(sharing = value) } }
    var permissionPending: AudioCapturePermission? = null
        private set
    var pending: AudioEndpoint?
        get() = mutableState.value.pending
        private set(value) {
            permissionPending = value?.let { AudioCapturePermission(it) }
            mutableState.update { it.copy(pending = value) }
        }
    private var visible = false
    private val statusUpdates = LatestDispatch(scope) { status = "Status update failed: ${it.message}" }
    private var generation = 0L
    private var observer: Job? = null
    private var backgroundRun: String? = null
    private var serviceError: Exception? = null
    private val retirement = ResourceLane(scope, { status = it })

    fun start() {
        visible = true
        if (observer?.isActive == true) return
        observer = scope.launch {
            invitations.collect { endpoint ->
                if (endpoint != null) { invitations.value = null; invite(endpoint) }
            }
        }
    }
    private fun invite(endpoint: AudioEndpoint) {
        stopSharing()
        if (!visible) return
        if (!endpoint.valid() || endpoint.direction != direction) { status = "Invalid audio request."; return }
        pending = endpoint
        if (!permission()) { requestPermission(); return }
        accept()
    }
    fun permissionResult(granted: Boolean, original: AudioCapturePermission?) {
        if (!visible || original == null || permissionPending !== original) return
        if (!granted) { pending = null; status = "Microphone permission denied."; return }
        accept()
    }
    fun accept() {
        val requested = pending ?: return
        if (!visible || !permission()) return
        try {
            val endpoint = preferences.effective(requested)
            pending = null
            launch(endpoint)
        } catch (error: Exception) { stopSharing(); status = error.message ?: "Audio unavailable." }
    }
    private fun launch(endpoint: AudioEndpoint) {
        retirement.checkAvailable()
        check(serviceError == null) { "Background cleanup incomplete: ${serviceError?.message}" }
        val revision = ++generation
        val settings = preferences
        if (endpoint.background) {
            backgroundRun = java.util.UUID.randomUUID().toString()
            service(backgroundRun)
        }
        sharing = true; status = "Starting $label…"
        retirement.replace({
            createRun(endpoint, settings) { text ->
                statusUpdates.submit { if (revision == generation) status = text }
            }
        }, { failure ->
            if (revision == generation) {
                if (failure != null) status = failure.message ?: "Audio stopped."
                sharing = false
                stopBackground()
            }
        })
    }
    private fun stopBackground() {
        if (backgroundRun == null) return
        try {
            service(null)
            backgroundRun = null
            serviceError = null
        } catch (error: Exception) {
            serviceError = error
            status = "Background cleanup incomplete: ${error.message}"
        }
    }
    fun stopSharing() {
        generation++
        pending = null
        retirement.stop()
        sharing = false
        stopBackground()
        val failure = retirement.error ?: serviceError
        status = failure?.let { "Cleanup incomplete: ${it.message}" }
            ?: if (retirement.stalled) "Native cleanup stalled; replacement waiting."
            else "${label.replaceFirstChar { it.uppercase() }} sharing is off. Start again on your computer."
    }
    fun configure(value: AudioPreferences) {
        require(value.valid(direction))
        stopSharing(); save(value); preferences = value
    }
    fun stop() {
        visible = false
        observer?.cancel(); observer = null
        pending = null
        if (backgroundRun == null) stopSharing()
    }
    fun ownsBackground(run: String?): Boolean = run != null && run == backgroundRun
    fun backgroundStopped(run: String?) { if (ownsBackground(run)) stopSharing() }
    fun shutdown() { stop(); stopSharing(); scope.cancel() }
}
