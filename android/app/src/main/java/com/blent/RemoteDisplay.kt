package com.blent

import android.content.Context
import android.view.Surface
import android.view.SurfaceView
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow

/** Activity adapter. Format/Surface replacements share one bounded retirement lane. */
internal class RemoteDisplay(context: Context,
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate),
    private val worker: (MediaTask.Display, Surface, (WorkerEvent) -> Unit) -> LifecycleRun = { task, surface, events -> AndroidWorkerRun(context, task, surface, events) },
) : DisplaySession<SurfaceView> {
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
    private val lane = ResourceLane(scope, { retirementStatus.value = it })
    private var wanted = false
    private var surface: Surface? = null
    @Volatile private var generation = 0L
    @Volatile private var connected = false
    @Volatile private var fps = 0f
    @Volatile private var mbps = 0f
    private var onConnected: () -> Unit = {}
    private var onDisconnected: () -> Unit = {}

    override fun observeConnection(connected: () -> Unit, disconnected: () -> Unit) {
        onConnected = connected; onDisconnected = disconnected
        if (this.connected) connected() else disconnected()
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
    override fun start() { if (!wanted) { wanted = true; replace() } }
    override fun stop() { wanted = false; replace() }
    private fun replace() {
        val revision = ++generation
        lane.stop()
        disconnect()
        val output = surface ?: return
        val selected = configuredRoutes ?: return
        if (selected.token == null) return
        if (!wanted) return
        startupStatus.value = "Starting isolated display worker…"
        val request = MediaTask.Display(selected, format)
        try {
            lane.replace({ worker(request, output) { event -> receive(revision, event) } }, { failure ->
                scope.launch {
                    if (generation == revision) {
                        generation++
                        wanted = false
                        disconnect()
                        startupStatus.value = failure?.message
                    }
                }
            })
        } catch (error: Exception) { startupStatus.value = error.message }
    }
    private fun receive(revision: Long, event: WorkerEvent) {
        if (generation != revision) return
        when (event) {
            is WorkerEvent.Rendered -> onFrameRendered?.invoke(RenderAcknowledgement(event.sequence, event.decodeMicros, event.decoder) { generation == revision })
            is WorkerEvent.Display -> snapshots.submit { if (generation == revision) publish(event) }
            else -> Unit
        }
    }
    private fun publish(event: WorkerEvent.Display) {
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
