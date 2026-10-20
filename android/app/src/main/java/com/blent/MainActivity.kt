package com.blent

import android.content.Intent
import android.hardware.usb.UsbManager
import com.blent.connection.*
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import kotlin.math.roundToInt
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    internal lateinit var connection: ConnectionBinding; private set
    internal lateinit var savedComputers: SavedTrustBinding; private set
    internal lateinit var configuredConnection: ConfiguredConnection; private set
    internal lateinit var microphone: AudioBinding; private set
    internal lateinit var speakers: AudioBinding; private set
    private val microphonePermission = registerForActivityResult(androidx.activity.result.contract.ActivityResultContracts.RequestPermission()) {
        microphonePermissionLauncher.complete(it)
    }
    private val microphonePermissionLauncher: LocalPermissionLauncher = LocalPermissionLauncher(
        { microphonePermission.launch(android.Manifest.permission.RECORD_AUDIO) },
        { prompt, granted -> AudioOwner.permissionResult(prompt, granted) },
    )
    private var audioAttachment: LocalPermissionAttachment? = null
    internal lateinit var cameras: CameraBinding; private set
    private val cameraPermission = registerForActivityResult(androidx.activity.result.contract.ActivityResultContracts.RequestPermission()) {
        cameraPermissionLauncher.complete(it)
    }
    private val cameraPermissionLauncher: LocalPermissionLauncher = LocalPermissionLauncher(
        { cameraPermission.launch(android.Manifest.permission.CAMERA) },
        { prompt, granted -> CameraOwner.permissionResult(prompt, granted) },
    )
    private var cameraAttachment: LocalPermissionAttachment? = null
    internal lateinit var session: SessionCoordinator; private set
    private var displayObservation: Job? = null
    private val displayRoutes = ConnectionDisplayBinding(DisplayRouteTarget { session.selectDisplay(it) })
    private lateinit var powerBinding: StreamingPowerBinding
    internal lateinit var windowPolicy: ActivityWindowPolicy; private set

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val prefs = Prefs(this)
        connection = ConnectionOwner.get(applicationContext)
        savedComputers = ConnectionOwner.getSaved(applicationContext)
        configuredConnection = ConfiguredConnection(connection, AndroidConnectionPreferences.get(applicationContext), lifecycleScope)
        cameras = CameraOwner.get(applicationContext)
        val cameraAccess = checkNotNull(CameraOwner.access)
        microphone = AudioOwner.get(applicationContext)
        speakers = AudioOwner.get(applicationContext, 2)
        val microphoneAccess = checkNotNull(AudioOwner.access(1))
        val speakerAccess = checkNotNull(AudioOwner.access(2))
        powerBinding = StreamingPowerBinding(this)
        windowPolicy = ActivityWindowPolicy(this, prefs)
        windowPolicy.applyDisplaySettings()
        windowPolicy.applyOrientation()
        session = SessionCoordinator(prefs, { runOnUiThread(it) },
            videoReceiver = RemoteDisplay(applicationContext),
            touchCapture = TouchCapture(capabilities = { width, height, fps -> RemoteMetadata.report(applicationContext, width, height, fps) }))
        reportNativeResolution()
        setContent {
            BlentTheme {
                BlentMain(
                    penOnly = session.penOnlyMode,
                    updateAvailable = session.updateAvailable,
                    showThanks = session.showThanks,
                    onDismissThanks = session::dismissThanks,
                    presentation = session.presentation,
                    settings = session.settings,
                    onSurfaceReady = session::surfaceReady,
                    onSurfaceDestroyed = session::surfaceDestroyed,
                    displayRefreshRates = windowPolicy.supportedDisplayModes().map { it.refreshRate },
                    onSettingsEvent = ::settingsEvent,
                    connectionControls = { ConnectionControls(connection, configuredConnection, savedComputers) },
                    cameraControls = { CameraControls(cameras, cameraAccess); AudioControls(microphone, microphoneAccess); AudioControls(speakers, speakerAccess) },
                    inputControls = { session.InputControls() },
                )
                ConnectionPairingDialog(connection)
            }
        }
        accessoryIntent(intent)
        window.decorView.post { windowPolicy.enableImmersiveMode() }
    }

    private fun settingsEvent(event: SettingsEvent) {
        session.handle(event)
        when (event) {
            is SettingsEvent.Orientation -> windowPolicy.applyOrientation()
            is SettingsEvent.Brightness, is SettingsEvent.RefreshRate -> windowPolicy.applyDisplaySettings()
            else -> Unit
        }
    }

    private fun reportNativeResolution() {
        // Report the real screen size (landscape-oriented) so the host can
        // size the virtual display to match this tablet exactly.
        @Suppress("DEPRECATION")
        val size = android.graphics.Point().also {
            windowManager.defaultDisplay.getRealSize(it)
        }
        if (size.x > 0 && size.y > 0) {
            val w = maxOf(size.x, size.y)
            val h = minOf(size.x, size.y)
            // Physical size too: the host puts it in the EDID, and the desktop
            // derives its DPI — and therefore its default scale — from that.
            // Paired long-edge-to-long-edge so it matches the w/h above
            // regardless of the panel's natural orientation.
            val dm = resources.displayMetrics
            val mmA = if (dm.xdpi > 1f) size.x / dm.xdpi * 25.4f else 0f
            val mmB = if (dm.ydpi > 1f) size.y / dm.ydpi * 25.4f else 0f
            val wMm = maxOf(mmA, mmB).roundToInt()
            val hMm = minOf(mmA, mmB).roundToInt()
            session.nativeResolution(w, h, wMm, hMm)
        }

    }

    override fun onNewIntent(intent: Intent?) {
        super.onNewIntent(intent)
        if (!accessoryIntent(intent)) session.applyToken(restart = true)
    }

    private fun accessoryIntent(intent: Intent?): Boolean {
        if (intent?.action != UsbManager.ACTION_USB_ACCESSORY_ATTACHED) return false
        configuredConnection.connect()
        return true
    }

    override fun onGenericMotionEvent(event: android.view.MotionEvent): Boolean =
        session.hover(event, window.decorView.width, window.decorView.height) || super.onGenericMotionEvent(event)

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            windowPolicy.enableImmersiveMode()
            windowPolicy.applyDisplaySettings()
        }
    }

    override fun onStart() {
        super.onStart()
        cameraAttachment = CameraOwner.attach(cameraPermissionLauncher::launch)
        audioAttachment = AudioOwner.attach(microphonePermissionLauncher::launch)
        windowPolicy.start()
        windowPolicy.startWake(session.powerDemandNow(), session.powerDemandUpdates())
        powerBinding.start(session.powerDemandNow(), session.powerDemandUpdates())
        session.checkUpdate {
            try { packageManager.getPackageInfo(packageName, 0).versionName ?: "0" } catch (_: Exception) { "0" }
        }
        displayRoutes.apply(connection.state.value)
        session.start()
        displayObservation?.cancel()
        displayObservation = lifecycleScope.launch { connection.state.collect { displayRoutes.apply(it) } }
    }

    override fun onStop() {
        displayObservation?.cancel(); displayObservation = null
        reportCleanup(cleanupAll({ detachCamera() }, { detachAudio() },
            { super.onStop() }, { windowPolicy.stop() }, { powerBinding.stop() }, { session.stop() }))
    }

    override fun onDestroy() {
        displayObservation?.cancel(); displayObservation = null
        reportCleanup(cleanupAll({ detachCamera(); cameraPermissionLauncher.retire() },
            { detachAudio(); microphonePermissionLauncher.retire() },
            { if (::powerBinding.isInitialized) powerBinding.stop() },
            { super.onDestroy() }, { windowPolicy.stop() }))
    }

    private fun detachCamera() {
        cameraAttachment?.let(CameraOwner::detach)
        cameraAttachment = null
    }

    private fun detachAudio() {
        audioAttachment?.let(AudioOwner::detach)
        audioAttachment = null
    }

    private fun reportCleanup(failures: List<Exception>) {
        failures.forEach { android.util.Log.w("BlentLifecycle", "Cleanup incomplete", it) }
    }
}
