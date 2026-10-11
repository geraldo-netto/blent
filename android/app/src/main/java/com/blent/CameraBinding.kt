package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/** Portable host-command, permission and resource ownership policy. */
internal class CameraBinding(
    private val rotation: () -> Int,
    private val requestPermission: () -> Unit,
    private val permission: () -> Boolean,
    private val createRun: (CameraEndpoint, CameraLens, () -> Int, (String) -> Unit) -> LifecycleRun,
    private val invitations: StateFlow<CameraEndpoint?>,
    private val scope: CoroutineScope,
    private val backgroundService: (String?) -> Unit,
) {
    private val mutableState = kotlinx.coroutines.flow.MutableStateFlow(CameraLifecycleState())
    val state = mutableState.asStateFlow()
    var endpoint: CameraEndpoint?
        get() = mutableState.value.endpoint
        private set(value) { mutableState.update { it.copy(endpoint = value) } }
    var selected: CameraLens?
        get() = mutableState.value.selected
        private set(value) { mutableState.update { it.copy(selected = value) } }
    var status: String
        get() = mutableState.value.status
        private set(value) { mutableState.update { it.copy(status = value) } }
    private val source = CameraCaptureSource()
    private var active = false
    private var backgroundRun: String? = null
    private val backgroundRunning get() = backgroundRun != null
    private var pending: Pair<CameraEndpoint, CameraLens>? = null
    private var observer: Job? = null
    private val statusUpdates = LatestDispatch(scope) { status = "Status update failed: ${it.message}" }
    private var generation = 0L
    private var serviceError: Exception? = null
    private val guard = ResourceLane(scope, { status = it })

    fun start() {
        if (active) return
        active = true
        source.target?.endpoint?.requestedLens?.let { if (selected == null) choose(it) }
        if (observer?.isActive == true) return
        observer = scope.launch {
            invitations.collect { value ->
                source.invitation(value)
                if (source.target == null && endpoint != source.endpoint) applyHost(source.endpoint)
            }
        }
    }

    /** Selected external ownership is explicit; ADB remains a separate source. */
    fun useTarget(value: CameraCaptureTarget?) {
        val previous = source.target
        if (!source.target(value)) return
        stopTarget(previous)
        applyHost(source.endpoint)
    }

    private fun applyHost(value: CameraEndpoint?) {
        stopCapture()
        endpoint = value
        pending = null
        status = if (value == null) "Start camera sharing on your computer." else "Camera sharing is off."
        val lens = value?.requestedLens
        if (lens == null) background(false) else choose(lens)
    }

    private fun stopTarget(value: CameraCaptureTarget?) {
        try { value?.authority?.stop() }
        catch (failure: Exception) { status = "Camera control failed: ${failure.message}" }
    }

    fun choose(lens: CameraLens?) {
        stopCapture()
        pending = null
        status = idleStatus()
        if (lens == null) { stopTarget(source.target); background(false); return }
        if (!active && !backgroundRunning) return
        val host = endpoint ?: return
        if (!active && !host.background) { background(false); return }
        if (!allowCapture(host, lens)) return
        try {
            guard.checkAvailable()
            check(serviceError == null) { "Background cleanup incomplete: ${serviceError?.message}" }
            background(host.background)
            launch(host, lens)
        } catch (error: Exception) {
            selected = null
            status = error.message ?: "Camera background service unavailable."
            stopTarget(source.target)
            background(false)
        }
    }

    private fun allowCapture(host: CameraEndpoint, lens: CameraLens): Boolean {
        if (permission()) return true
        if (source.target != null) {
            // External offers obtain permission before admission. Permission loss
            // after admission closes that original target, never a legacy prompt.
            stopTarget(source.target)
            status = "Camera permission unavailable."
            return false
        }
        background(false)
        if (!active) { status = "Open Blent on the tablet to allow camera access."; return false }
        pending = host to lens
        requestPermission()
        return false
    }

    fun permissionResult(granted: Boolean) {
        val request = pending
        pending = null
        if (!active || request == null) return
        if (!granted) { status = "Camera permission denied."; return }
        if (request.first != endpoint) { status = "Computer connection changed; select camera again."; return }
        choose(request.second)
    }

    private fun launch(host: CameraEndpoint, lens: CameraLens) {
        val revision = generation
        selected = lens
        status = "${lens.label} camera selected."
        val target = source.target
        val claim = target?.authority?.retain()
        check(target == null || claim != null) { "Camera target no longer current" }
        try {
            guard.replace({ CameraCaptureRun(claim, permission) {
                createRun(host, lens, rotation) { text ->
                    statusUpdates.submit { if (revision == generation) status = text }
                }
            } }, { failure ->
                if (generation == revision) {
                    stopTarget(target)
                    if (failure != null) status = failure.message ?: "Camera sharing failed."
                    selected = null
                    background(false)
                }
            }, { proof -> claim?.retired(proof) })
        } catch (failure: Exception) {
            claim?.retired(ResourceRetirement.NOT_STARTED)
            throw failure
        }
    }

    private fun idleStatus(): String = guard.error?.let { "Native cleanup incomplete: ${it.message}" }
        ?: if (guard.stalled) "Native cleanup stalled; replacement waiting." else "Camera sharing is off."

    private fun stopCapture() {
        generation++
        guard.stop()
        selected = null
    }

    private fun background(enabled: Boolean) {
        if (backgroundRunning == enabled) return
        if (!enabled) { stopBackground(); return }
        backgroundRun = java.util.UUID.randomUUID().toString()
        backgroundService(backgroundRun)
    }

    private fun stopBackground() {
        try {
            backgroundService(null)
            backgroundRun = null
            serviceError = null
        } catch (error: Exception) {
            serviceError = error
            status = "Background cleanup incomplete: ${error.message}"
        }
    }

    fun stop() {
        active = false
        pending = null
        if (backgroundRunning && selected != null) return
        shutdown()
    }

    fun ownsBackground(run: String?): Boolean = run != null && backgroundRun == run

    fun backgroundStopped(run: String?) {
        if (ownsBackground(run)) shutdown()
    }

    fun shutdown() {
        active = false
        observer?.cancel()
        observer = null
        pending = null
        stopTarget(source.target)
        source.target(null)
        stopCapture()
        endpoint = null
        status = idleStatus()
        background(false)
    }
}
