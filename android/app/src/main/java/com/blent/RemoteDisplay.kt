package com.blent

import android.content.Context
import android.view.Surface
import android.view.SurfaceView
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow

/** Activity presentation adapter. Every instance shares the application's
 * original display-worker retirement fence; callbacks stay client-bound. */
internal class RemoteDisplay(context: Context,
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate),
    initialGeneration: Long = 0,
    private val workers: DisplayWorkerOwner = (context.applicationContext as BlentApplication).displayWorkers,
    private val retryWait: suspend (Long) -> Unit = { delay(it) },
    private val worker: (MediaTask.Display, Surface, (WorkerEvent) -> Unit) -> LifecycleRun = { task, surface, events -> AndroidWorkerRun(context, task, surface, events) },
) : DisplaySession<SurfaceView> {
    private var liveness = false
    private var format = DecoderFormat(VideoWire.MIME_TYPE, 1920, 1080, Prefs.DEFAULT_FPS)
    private var configuredRoutes: DisplayRoutes? = AdbDisplayRoutes.create()
    private var routeRevision = 1L
    override var routes: DisplayRoutes?
        get() = configuredRoutes
        set(value) {
            if (configuredRoutes == value) return
            require(value == null || value.revision > routeRevision)
            configuredRoutes = value
            if (value != null) routeRevision = value.revision
            replace()
        }
    override var token: String?
        get() = configuredRoutes?.token
        set(value) { configuredRoutes = configuredRoutes?.copy(token = value) }
    override var mimeType: String
        get() = format.mimeType
        set(value) { setStreamFormat(format.copy(mimeType = value)) }
    override var streamFps: Int
        get() = format.fps
        set(value) { if (value in 10..90) setStreamFormat(format.copy(fps = value)) }
    @Volatile override var onFrameRendered: ((RenderAcknowledgement) -> Unit)? = null
    override val startupStatus = MutableStateFlow<String?>(null)
    override val retirementStatus = MutableStateFlow<String?>(null)
    override val decoderDiagnostics = MutableStateFlow(DecoderDiagnostics())
    private val snapshots = LatestDispatch(scope) { startupStatus.value = "Display status update failed: ${it.message}" }
    private var client: Any? = null
    private var wanted = false
    private var retry: Job? = null
    private var retryMillis = 500L
    private var surface: Surface? = null
    private val generations = GenerationFence(initialGeneration)
    @Volatile private var connected = false
    @Volatile private var fps = 0f
    @Volatile private var mbps = 0f
    private var onConnected: () -> Unit = {}
    private var onDisconnected: () -> Unit = {}

    override fun observeConnection(connected: () -> Unit, disconnected: () -> Unit) {
        onConnected = connected; onDisconnected = disconnected
        if (this.connected) connected() else disconnected()
    }
    override fun setVideoLiveness(enabled: Boolean) {
        if (liveness == enabled) return
        liveness = enabled
        replace()
    }
    override fun setStreamFormat(format: DecoderFormat) {
        if (this.format == format) return
        this.format = format
        replace()
    }
    override fun setSurface(surfaceView: SurfaceView) {
        val value = surfaceView.holder.surface
        if (value?.isValid != true) return
        if (surface === value) return
        surface = value
        replace()
    }
    override fun onSurfaceDestroyed() { surface = null; replace() }
    override fun start() {
        if (wanted && workers.owns(client) && retry == null) return
        client = workers.open()
        wanted = true
        retryMillis = 500L
        replace()
    }
    override fun stop() { wanted = false; retryMillis = 500L; replace() }
    private fun replace() {
        retry?.cancel()
        retry = null
        generations.revoke()
        workers.clear(client)
        disconnect()
        val output = surface ?: return
        val selected = configuredRoutes ?: return
        if (selected.token == null) return
        if (!wanted || !workers.owns(client)) return
        val revision = generations.acquire()
        if (revision == null) {
            wanted = false
            startupStatus.value = "Display generations exhausted"
            return
        }
        startupStatus.value = "Starting isolated display worker…"
        val request = MediaTask.Display(selected, format, liveness)
        val status = retirementStatus
        try {
            workers.replace(checkNotNull(client), { events -> worker(request, output, events) },
                { event -> receive(revision, event) }, { failure -> ended(revision, failure) },
                { status.value = it })
        } catch (error: Exception) { startupStatus.value = error.message }
    }
    private fun current(revision: Long): Boolean = generations.current(revision) && workers.owns(client)
    private fun ended(revision: Long, failure: Exception?) {
        scope.launch {
            if (current(revision)) {
                generations.revoke()
                disconnect()
                startupStatus.value = failure?.message
                retryAfterExit()
            }
        }
    }
    private fun retryAfterExit() {
        if (!wanted || !generations.available()) {
            wanted = false
            return
        }
        val wait = retryMillis
        retryMillis = (wait * 2).coerceAtMost(30_000L)
        retry = scope.launch(start = CoroutineStart.LAZY) {
            retryWait(wait)
            retry = null
            if (wanted && workers.owns(client)) replace()
        }
        retry?.start()
    }
    private fun receive(revision: Long, event: WorkerEvent) {
        if (!current(revision)) return
        when (event) {
            is WorkerEvent.Rendered -> onFrameRendered?.invoke(RenderAcknowledgement(event.sequence, event.decodeMicros, event.decoder) { current(revision) })
            is WorkerEvent.Display -> snapshots.submit { if (current(revision)) publish(event) }
            else -> Unit
        }
    }
    private fun publish(event: WorkerEvent.Display) {
        if (event.connected) retryMillis = 500L
        startupStatus.value = event.startup
        retirementStatus.value = event.retirement
        decoderDiagnostics.value = event.diagnostics
        fps = event.fps; mbps = event.mbps
        if (connected != event.connected) {
            connected = event.connected
            if (connected) onConnected() else onDisconnected()
        }
    }
    private fun disconnect() {
        connected = false; fps = 0f; mbps = 0f
        decoderDiagnostics.value = DecoderDiagnostics()
        startupStatus.value = null
        onDisconnected()
    }
    override fun getFps() = fps
    override fun getMbps() = mbps
}
