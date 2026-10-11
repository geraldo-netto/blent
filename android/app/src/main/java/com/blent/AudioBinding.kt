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
    init { require(initial.valid(direction)) { "Invalid initial audio direction/preferences" } }
    private var target: AudioConsumerTarget? = null
    private var targetStarted = false
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
        target?.let { if (!targetStarted) startTarget(it) }
        if (observer?.isActive == true) return
        observer = scope.launch {
            invitations.collect { endpoint ->
                if (endpoint != null) {
                    invitations.value = null
                    if (target == null) invite(endpoint)
                }
            }
        }
    }
    /** Selected external ownership remains explicit until withdrawal. ADB uses
     * its existing one-shot invitations; it cannot replace or revive this target. */
    fun useTarget(value: AudioConsumerTarget?) {
        require(value == null || value.endpoint.direction == direction) { "Audio target direction mismatch" }
        if (target === value) return
        stopSharing()
        target = value
        targetStarted = false
        if (value != null) startTarget(value)
    }
    private fun startTarget(value: AudioConsumerTarget) {
        if (!visible || targetStarted) return
        targetStarted = true
        try {
            check(permission()) { "Audio permission unavailable" }
            launch(preferences.effective(value.endpoint), value)
        } catch (error: Exception) {
            stopSharing()
            status = error.message ?: "Audio unavailable."
        }
    }
    private fun stopTarget(value: AudioConsumerTarget?) {
        try { value?.authority?.stop() }
        catch (error: Exception) { status = "Audio control failed: ${error.message}" }
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
        if (!visible || target != null || !permission()) return
        try {
            val endpoint = preferences.effective(requested)
            pending = null
            launch(endpoint, null)
        } catch (error: Exception) { stopSharing(); status = error.message ?: "Audio unavailable." }
    }
    private fun launch(endpoint: AudioEndpoint, owner: AudioConsumerTarget?) {
        retirement.checkAvailable()
        check(serviceError == null) { "Background cleanup incomplete: ${serviceError?.message}" }
        val claim = owner?.authority?.retain()
        check(owner == null || claim != null) { "Audio target no longer current" }
        try { launchOwned(endpoint, owner, claim) }
        catch (error: Exception) { claim?.retired(ResourceRetirement.NOT_STARTED); throw error }
    }
    private fun launchOwned(endpoint: AudioEndpoint, owner: AudioConsumerTarget?, claim: MediaConsumerClaim?) {
        val revision = ++generation
        val settings = preferences
        if (endpoint.background) {
            backgroundRun = java.util.UUID.randomUUID().toString()
            service(backgroundRun)
        }
        sharing = true; status = "Starting $label…"
        retirement.replace({
            val create = {
                createRun(endpoint, settings) { text ->
                    statusUpdates.submit { if (revision == generation) status = text }
                }
            }
            if (owner == null) create() else MediaConsumerRun(claim, permission, create)
        }, { failure ->
            if (revision == generation) {
                stopTarget(owner)
                if (failure != null) status = failure.message ?: "Audio stopped."
                sharing = false
                stopBackground()
            }
        }, { proof -> claim?.retired(proof) })
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
        targetStarted = true
        stopTarget(target)
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
    fun shutdown() {
        stop(); stopSharing(); target = null; scope.cancel()
    }
}
