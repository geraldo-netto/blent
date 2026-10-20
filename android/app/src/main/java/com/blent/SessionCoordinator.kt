package com.blent

import com.blent.activity.DisplayActivityState
import com.blent.activity.DisplayActivityUpdate
import com.blent.activity.desired
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.compose.runtime.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flowOf

/** Read-only stream observations for Compose; owning a view never starts video. */
internal class StreamPresentation(private val receiver: DisplaySession<android.view.SurfaceView>?, control: TouchCapture?) {
    var connected by mutableStateOf(false); private set
    var fps by mutableStateOf(0f); private set
    var mbps by mutableStateOf(0f); private set
    val decoderStartup = receiver?.startupStatus ?: MutableStateFlow<String?>(null)
    val decoderRetirement = receiver?.retirementStatus ?: MutableStateFlow<String?>(null)
    val decoderDiagnostics = receiver?.decoderDiagnostics ?: MutableStateFlow(DecoderDiagnostics())
    val connectionState = control?.connectionState ?: MutableStateFlow(ControlConnection())
    val controlConnected: StateFlow<Boolean> = control?.controlConnected ?: MutableStateFlow(false)
    val calibrating = control?.control?.calibrating ?: MutableStateFlow(false)
    val directInput = control?.motion?.direct?.state ?: MutableStateFlow<DirectControlsState?>(null)
    init {
        val ui = Handler(Looper.getMainLooper())
        // Always enqueue, preserving worker/UI callback ordering (T300/T329).
        receiver?.observeConnection(
            { ui.post { connected = true } }, { ui.post { connected = false } },
        )
    }
    fun sample() { fps = receiver?.getFps() ?: 0f; mbps = receiver?.getMbps() ?: 0f }
}

internal data class SettingsValues(
    val bitrateKbps: Int = Prefs.DEFAULT_BITRATE_KBPS,
    val fps: Int = Prefs.DEFAULT_FPS,
    val brightness: Int = Prefs.DEFAULT_BRIGHTNESS_PERCENT,
    val refreshRate: Float = Prefs.DEFAULT_DISPLAY_REFRESH_RATE,
    val orientation: Int = Prefs.ORIENTATION_AUTO,
    val showStats: Boolean = false,
    val checkUpdates: Boolean = true,
    val batterySaver: Boolean = false,
    val streamError: String? = null,
    val controlQueueBytes: Int = ControlCapacity.DEFAULT,
) {
    companion object {
        fun read(prefs: Prefs) = SettingsValues(prefs.bitrateKbps, prefs.fps, prefs.brightnessPercent,
            prefs.displayRefreshRate, prefs.orientation, prefs.showStats, prefs.checkUpdates, prefs.batterySaver, controlQueueBytes = prefs.controlQueueBytes)
    }
}

internal sealed interface SettingsEvent {
    data class Stream(val bitrate: Int, val fps: Int) : SettingsEvent
    data class Mode(val penOnly: Boolean) : SettingsEvent
    data class Orientation(val value: Int) : SettingsEvent
    data class Brightness(val value: Int) : SettingsEvent
    data class RefreshRate(val value: Float) : SettingsEvent
    data class ShowStats(val value: Boolean) : SettingsEvent
    data class BatterySaver(val value: Boolean) : SettingsEvent
    data class CheckUpdates(val value: Boolean) : SettingsEvent
    data class ControlQueue(val bytes: Int) : SettingsEvent
}

/** One owner per Activity instance. Rendering observes state and emits events. */
internal class SessionCoordinator(
    private val prefs: Prefs,
    private val dispatchUi: (() -> Unit) -> Unit,
    private var videoReceiver: DisplaySession<android.view.SurfaceView>? = VideoReceiver(),
    private var touchCapture: TouchCapture? = TouchCapture(),
    releaseChecks: ReleaseChecks = UpdateCheck.requests,
    private val routeSequence: DisplayRouteSequence = DisplayRouteSequence(),
) : DisplayRouteTarget {
    var penOnlyMode by mutableStateOf(false); private set
    var showThanks by mutableStateOf(false); private set
    var updateAvailable by mutableStateOf<String?>(null); private set
    var settings by mutableStateOf(SettingsValues.read(prefs)); private set
    private val updates = ReleaseCheckOwner(releaseChecks, { prefs.checkUpdates }, dispatchUi) { updateAvailable = it }
    private var appliedActivity by mutableStateOf<DisplayActivityUpdate?>(null)
    private var modeSource by mutableStateOf<Any?>(null)
    private val activityDelivery = DisplayActivityDelivery(dispatchUi, ::applyDisplayActivity) { failure ->
        Log.w("Blent", "Display activity UI unavailable", failure)
    }
    private var started by mutableStateOf(false)
    private var stopTokenObservation: (() -> Unit)? = null
    private data class DecoderSupport(val format: Boolean = true, val codec: Boolean = true) {
        val available get() = format && codec
    }
    private var decoderSupport by mutableStateOf(DecoderSupport())
    private var externalDisplay = false
    private var externalAvailable = false
    val presentation = StreamPresentation(videoReceiver, touchCapture)

    private data class WakeLocal(
        val saver: Boolean, val video: Boolean, val pen: Boolean, val foreground: Boolean,
        val applied: DisplayActivityUpdate?, val mode: Any?, val decoder: Boolean,
    )
    private fun wakeLocal() = WakeLocal(settings.batterySaver, presentation.connected, penOnlyMode,
        started, appliedActivity, modeSource, decoderSupport.available)
    fun powerDemandNow() = powerFor(wakeLocal(), presentation.connectionState.value)
    fun powerNow(): StreamingPower = powerDemandNow().power
    fun powerDemandUpdates() = combine(
        snapshotFlow { wakeLocal() }, presentation.connectionState,
        touchCapture?.control?.displayActivityHints ?: flowOf(null),
    ) { local, connection, _ -> powerFor(local, connection) }.distinctUntilChanged()

    private fun powerFor(local: WakeLocal, connection: ControlConnection): DisplayPowerDemand {
        val source = touchCapture?.control?.displayActivityView
        val update = source?.update
        val state = DisplayWakeSnapshot(local.foreground, connection.authenticated, metadataCurrent(local, update), update).wakeState()
        val permit = if (source == null || update == null) FixedDisplayPowerPermit else ActivityPowerPermit(source, update)
        return DisplayPowerDemand(powerValues(local, connection, state), permit)
    }
    private fun metadataCurrent(local: WakeLocal, update: DisplayActivityUpdate?): Boolean =
        update != null && local.applied === update && local.mode === update.source && (local.pen || local.decoder)
    private fun powerValues(local: WakeLocal, connection: ControlConnection, state: DisplayWakeState) =
        StreamingPower(local.saver, connection.authenticated && (local.video || local.pen), connection.transport, state)

    init { touchCapture?.setQueueBudget(prefs.controlQueueBytes); applyToken(false); connectStreamCallbacks() }

    fun handle(event: SettingsEvent) {
        when (event) {
            is SettingsEvent.Stream -> requestStreamSettings(event)
            is SettingsEvent.Mode -> touchCapture?.sendMode(event.penOnly)
            is SettingsEvent.CheckUpdates -> {
                prefs.checkUpdates = event.value
                updates.preferenceChanged()
            }
            else -> localPreference(event)
        }
        settings = SettingsValues.read(prefs).copy(streamError = settings.streamError)
    }

    private fun localPreference(event: SettingsEvent) {
        when (event) {
            is SettingsEvent.Orientation -> prefs.orientation = event.value
            is SettingsEvent.Brightness -> prefs.brightnessPercent = event.value
            is SettingsEvent.RefreshRate -> prefs.displayRefreshRate = event.value
            is SettingsEvent.ShowStats -> prefs.showStats = event.value
            is SettingsEvent.BatterySaver -> prefs.batterySaver = event.value
            is SettingsEvent.ControlQueue -> {
                prefs.controlQueueBytes = event.bytes
                touchCapture?.setQueueBudget(prefs.controlQueueBytes)
            }
            else -> Unit
        }
    }

    private fun requestStreamSettings(event: SettingsEvent.Stream) {
        if (!displayAllowed()) videoReceiver?.stop()
        settings = settings.copy(streamError = null)
        applyStreamSettings(prefs, touchCapture, videoReceiver, event.bitrate, event.fps)
    }

    fun dismissThanks() { showThanks = false }
    @Composable fun InputControls() { DirectInputControls(touchCapture) }
    fun surfaceReady(view: android.view.SurfaceView) {
        videoReceiver?.setSurface(view)
        touchCapture?.setSurfaceView(view)
    }
    fun surfaceDestroyed() { videoReceiver?.onSurfaceDestroyed() }
    fun hover(event: android.view.MotionEvent, width: Int, height: Int): Boolean =
        width > 0 && height > 0 && touchCapture?.handleHoverEvent(event, width, height) == true

    fun nativeResolution(width: Int, height: Int, widthMm: Int, heightMm: Int) {
        touchCapture?.setNativeResolution(width, height, widthMm, heightMm)
    }

    fun start() {
        started = true
        activityDelivery.start()
        updates.start()
        stopTokenObservation?.invoke()
        stopTokenObservation = prefs.observeHostToken { if (started) applyToken(restart = true) }
        applyToken(restart = false)
        connectControl()
    }
    private fun connectControl() {
        touchCapture?.connect()
        if (prefs.hasUserSettings) touchCapture?.sendConfig(prefs.bitrateKbps, prefs.fps)
    }
    fun stop() {
        started = false
        activityDelivery.stop()
        appliedActivity = null
        modeSource = null
        updates.stop()
        stopTokenObservation?.invoke()
        stopTokenObservation = null
        videoReceiver?.stop()
        touchCapture?.disconnect()
    }
    fun checkUpdate(currentVersion: () -> String) {
        updates.check(currentVersion())
    }

    private fun connectStreamCallbacks() {
        touchCapture?.onDisplayActivityKnown = { update -> activityDelivery.submit(update) }
        // Close the host's latency measurement loop: every acknowledged frame
        // lets the host time encoded-packet readiness to receipt of the
        // render acknowledgement, including the return message path.
        videoReceiver?.onFrameRendered = { receipt ->
            touchCapture?.sendRendered(receipt)
            // First frame ever on screen: say thanks once, then never again.
            if (!prefs.thankedOnce) {
                prefs.thankedOnce = true
                dispatchUi { showThanks = true }
            }
        }
        videoReceiver?.streamFps = prefs.fps
        connectSettingsCallbacks()
        connectFormatCallbacks()
        touchCapture?.onModeKnown = { penOnly ->
            withCurrentControl {
                penOnlyMode = penOnly
                modeSource = touchCapture?.displayActivityUpdate?.source
                reconcileDisplay()
            }
        }

    }

    private fun connectFormatCallbacks() {
        touchCapture?.onVideoLivenessKnown = { enabled ->
            withCurrentControl { configureVideo { it.setVideoLiveness(enabled) } }
        }
        touchCapture?.onStreamFormat = { format ->
            withCurrentControl {
                decoderSupport = decoderSupport.copy(format = format != null)
                configureVideo { if (format != null) it.setStreamFormat(format) }
            }
        }
        touchCapture?.onCodecKnown = { codec ->
            withCurrentControl {
                val mime = VideoCodec.types[codec]
                decoderSupport = decoderSupport.copy(codec = mime != null)
                if (mime == null) {
                    Log.w("Blent", "Unsupported host video codec: $codec")
                    videoReceiver?.stop()
                    return@withCurrentControl
                }
                val vr = videoReceiver
                if (vr != null && vr.mimeType != mime) {
                    Log.i("Blent", "Host is sending $codec — rebuilding the decoder")
                    if (!penOnlyMode) vr.stop()
                    vr.mimeType = mime
                    reconcileDisplay()
                }
            }
        }
    }

    private fun connectSettingsCallbacks() {
        touchCapture?.onFpsKnown = { fps ->
            withCurrentControl { configureVideo { it.streamFps = fps } }
        }
        touchCapture?.onSettingsRejected = { rejected ->
            withCurrentControl {
                if (touchCapture?.settingsGeneration == rejected.requestGeneration) {
                    restoreStreamSettings(rejected.reason, rejected.bitrate, rejected.fps)
                }
            }
        }
    }

    private fun restoreStreamSettings(reason: String, bitrate: Int, fps: Int) {
        prefs.bitrateKbps = bitrate
        prefs.fps = fps
        configureVideo { it.streamFps = fps }
        settings = SettingsValues.read(prefs).copy(streamError = reason)
    }

    private fun applyDisplayActivity(update: DisplayActivityUpdate) {
        val source = touchCapture ?: return
        if (!started || source.displayActivityUpdate !== update) return
        val previous = appliedActivity
        if (previous?.source !== update.source || interrupted(previous, update)) videoReceiver?.stop()
        appliedActivity = update
        reconcileDisplay()
    }
    private fun interrupted(previous: DisplayActivityUpdate?, next: DisplayActivityUpdate): Boolean {
        val old = (previous?.state as? DisplayActivityState.Observed)?.signal
        val current = (next.state as? DisplayActivityState.Observed)?.signal
        return old?.stopped != current?.stopped
    }
    private fun currentActivity(): DisplayActivityUpdate? {
        val source = touchCapture ?: return null
        val update = appliedActivity ?: return null
        if (source.displayActivityUpdate !== update || !source.connectionState.value.authenticated) return null
        return update
    }
    private fun displayAllowed(): Boolean = started && !penOnlyMode && decoderSupport.available && currentActivity().let {
        it != null && modeSource === it.source && it.state.desired
    }
    private fun reconcileDisplay() {
        if (displayAllowed()) videoReceiver?.start() else videoReceiver?.stop()
    }
    private fun configureVideo(change: (DisplaySession<android.view.SurfaceView>) -> Unit) {
        val receiver = videoReceiver ?: return
        if (!displayAllowed()) receiver.stop()
        change(receiver)
        reconcileDisplay()
    }

    // A greeting can already be queued on the UI thread when onStop or a
    // token change retires its socket. Validate again when the action runs.
    private fun withCurrentControl(action: () -> Unit) {
        val source = touchCapture ?: return
        val generation = source.connectionGeneration
        dispatchUi {
            if (started && touchCapture === source && source.connectionGeneration == generation) action()
        }
    }

    fun applyToken(restart: Boolean) {
        val token = prefs.hostToken
        if (externalDisplay) { legacyAfterExternal(token, restart); return }
        if (token == null) { retireToken(); return }
        val changed = touchCapture?.token != token
        touchCapture?.token = token
        videoReceiver?.token = token
        // Active reconnect loops also captured the previous token. Retire them
        // even when control is temporarily disconnected. Background sessions
        // stay stopped; onStart connects with the updated credentials.
        if (started && restart && changed) {
            Log.i("Blent", "New session token — reconnecting")
            touchCapture?.disconnect()
            touchCapture?.connect()
            if (!penOnlyMode && decoderSupport.available) {
                videoReceiver?.stop()
                reconcileDisplay()
            }
        }
    }

    override fun selectDisplay(endpoint: DisplayEndpoint?) {
        externalDisplay = true
        externalAvailable = false
        try {
            clearRoutes()
            if (endpoint == null) return
            installRoutes(routeSequence.next(endpoint.token, endpoint.videoPort, endpoint.controlPort))
            externalAvailable = true
        } catch (failure: Exception) { routeFailure(failure) }
    }
    private fun clearRoutes() {
        val failures = cleanupAll({ touchCapture?.routes = null }, { videoReceiver?.routes = null },
            { touchCapture?.disconnect() }, { videoReceiver?.stop() })
        check(failures.isEmpty()) { "Display route cleanup incomplete" }
    }
    private fun installRoutes(routes: DisplayRoutes) {
        touchCapture?.routes = routes
        videoReceiver?.routes = routes
        // Codec and mode come from the new control greeting before video starts.
        decoderSupport = DecoderSupport()
        settings = settings.copy(streamError = null)
        if (started) connectControl()
    }
    private fun routeFailure(failure: Exception) {
        cleanupAll({ touchCapture?.routes = null }, { videoReceiver?.routes = null },
            { touchCapture?.disconnect() }, { videoReceiver?.stop() })
        externalAvailable = false
        settings = settings.copy(streamError = failure.message ?: "Display route unavailable")
    }
    private fun legacyAfterExternal(token: String?, restart: Boolean) {
        // Fresh legacy delivery may select ADB after USB revocation. Background
        // resume and persisted credentials alone never restore a revoked route.
        if (externalAvailable || !restart || token == null) return
        try {
            clearRoutes()
            val adb = AdbDisplayRoutes.create(token)
            installRoutes(routeSequence.next(token, adb.videoPort, adb.controlPort))
            externalDisplay = false
        } catch (failure: Exception) { routeFailure(failure) }
    }

    private fun retireToken() {
        // Null at initial ADB startup still permits its token-delivery handshake.
        // Once credentials existed, clearing them must retire their captured work.
        if (touchCapture?.token == null && videoReceiver?.token == null) return
        touchCapture?.disconnect()
        videoReceiver?.stop()
        touchCapture?.token = null
        videoReceiver?.token = null
    }

}

internal fun applyStreamSettings(prefs: Prefs?, touchCapture: TouchCapture?, videoReceiver: DisplaySession<android.view.SurfaceView>?, bitrateKbps: Int, newFps: Int) {
    prefs?.bitrateKbps = bitrateKbps
    prefs?.fps = newFps
    prefs?.hasUserSettings = true
    videoReceiver?.streamFps = newFps
    touchCapture?.sendConfig(bitrateKbps, newFps)
}
