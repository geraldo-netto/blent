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
    private val createRun: (CameraEndpoint, CameraLens, () -> Int) -> LifecycleRun,
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
    private var active = false
    private var backgroundRun: String? = null
    private val backgroundRunning get() = backgroundRun != null
    private var pending: Pair<CameraEndpoint, CameraLens>? = null
    private var observer: Job? = null
    private var worker: Job? = null
    private val retirementScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var retirement: Job? = null
    private var resources: LifecycleRun? = null
    private var generation = 0L
    private var serviceError: Exception? = null
    private val guard = RetirementGuard()

    fun start() {
        if (active) return
        active = true
        if (observer?.isActive == true) return
        observer = scope.launch {
            invitations.collect { value ->
                if (endpoint != value) {
                    stopCapture()
                    endpoint = value
                    pending = null
                    status = if (value == null) "Start camera sharing on your computer." else "Camera sharing is off."
                    choose(value?.requestedLens)
                }
            }
        }
    }

    fun choose(lens: CameraLens?) {
        val previous = stopCapture()
        pending = null
        status = guard.error?.let { "Native cleanup incomplete: ${it.message}" } ?: "Camera sharing is off."
        if (lens == null) { background(false); return }
        if (!active && !backgroundRunning) return
        val host = endpoint ?: return
        if (!active && !host.background) { background(false); return }
        if (!allowCapture(host, lens)) return
        try {
            guard.checkAvailable()
            check(serviceError == null) { "Background cleanup incomplete: ${serviceError?.message}" }
            background(host.background)
            launch(host, lens, previous)
        } catch (error: Exception) {
            status = error.message ?: "Camera background service unavailable."
            background(false)
        }
    }

    private fun allowCapture(host: CameraEndpoint, lens: CameraLens): Boolean {
        if (permission()) return true
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

    private fun launch(host: CameraEndpoint, lens: CameraLens, previous: Job?) {
        val revision = generation
        val owned = createRun(host, lens, rotation)
        resources = owned
        selected = lens
        status = "${lens.label} camera selected."
        worker = scope.launch {
            try {
                previous?.join()
                guard.checkAvailable()
                owned.execute()
            } catch (error: Exception) {
                if (generation == revision && error !is CancellationException) status = error.message ?: "Camera sharing failed."
            } finally {
                try {
                    if (generation == revision) { selected = null; background(false) }
                } finally {
                    withContext(NonCancellable + Dispatchers.IO) { retire(owned) }
                }
            }
        }
    }

    private fun stopCapture(): Job? {
        generation++
        val previous = worker
        val owned = resources
        val earlier = retirement
        worker = null
        resources = null
        cleanupAll({ owned?.cancel() }, { previous?.cancel() }).firstOrNull()?.let {
            status = "Cancellation incomplete: ${it.message}"
        }
        if (owned != null) retirement = retirementScope.launch {
            earlier?.join()
            previous?.join()
            retire(owned)
        }
        selected = null
        return retirement
    }

    private fun background(enabled: Boolean) {
        if (backgroundRunning == enabled) return
        if (!enabled) { stopBackground(); return }
        backgroundRun = java.util.UUID.randomUUID().toString()
        backgroundService(backgroundRun)
    }

    private suspend fun retire(owned: LifecycleRun) {
        guard.retire(owned)?.let { status = "Native cleanup incomplete: ${it.message}" }
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
        stopCapture()
        endpoint = null
        status = guard.error?.let { "Native cleanup incomplete: ${it.message}" } ?: "Camera sharing is off."
        background(false)
    }
}
