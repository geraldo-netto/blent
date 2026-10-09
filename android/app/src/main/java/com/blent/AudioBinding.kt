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
    var pending: AudioEndpoint?
        get() = mutableState.value.pending
        private set(value) { mutableState.update { it.copy(pending = value) } }
    private var visible = false
    private var generation = 0L
    private var observer: Job? = null
    private var worker: Job? = null
    private var resources: LifecycleRun? = null
    private var backgroundRun: String? = null
    private var serviceError: Exception? = null
    private val retirement = RetirementGuard()

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
    fun permissionResult(granted: Boolean) {
        if (!visible || pending == null) return
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
        val earlier = worker
        val revision = ++generation
        val owned = createRun(endpoint, preferences) { text ->
            scope.launch { if (revision == generation) status = text }
        }
        resources = owned
        if (endpoint.background) {
            backgroundRun = java.util.UUID.randomUUID().toString()
            service(backgroundRun)
        }
        sharing = true; status = "Starting $label…"
        worker = scope.launch {
            try {
                // A cancelled waiter must retain the chain to older native owners.
                withContext(NonCancellable) { earlier?.join() }
                currentCoroutineContext().ensureActive()
                retirement.checkAvailable()
                owned.execute()
            } catch (error: Exception) {
                if (revision == generation && error !is CancellationException) status = error.message ?: "Audio stopped."
            } finally {
                val failure = withContext(NonCancellable) { retirement.retire(owned) }
                if (revision == generation) { sharing = false; stopBackground() }
                if (failure != null) status = "Native cleanup incomplete: ${failure.message}"
            }
        }
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
        val stopping = resources; resources = null
        val failures = cleanupAll({ stopping?.cancel() }, { worker?.cancel() })
        sharing = false
        stopBackground()
        val failure = retirement.error ?: serviceError ?: failures.firstOrNull()
        status = failure?.let { "Cleanup incomplete: ${it.message}" }
            ?: "${label.replaceFirstChar { it.uppercase() }} sharing is off. Start again on your computer."
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
