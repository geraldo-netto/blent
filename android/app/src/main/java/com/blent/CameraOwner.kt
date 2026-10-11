package com.blent

import android.content.Context
import android.content.Intent
import android.hardware.display.DisplayManager
import android.view.Display
import android.os.Handler
import android.os.Looper
import com.blent.connection.ConnectionCameraAccess
import com.blent.connection.ConnectionOwner
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.android.asCoroutineDispatcher
import kotlinx.coroutines.cancel

/** Process camera owner holds only the application; Activity callbacks detach onStop. */
internal object CameraOwner {
    private var binding: CameraBinding? = null
    private var control: CameraActivityOwner? = null
    private val prompts = LocalPermissionPrompts()
    private var observation: MediaConsentObservation<CameraAccessOffer>? = null
    private var captureObservation: MediaConsumerObservation<CameraAccessOffer, CameraEndpoint, CameraCaptureTarget>? = null
    private var scope: CoroutineScope? = null
    val access: MediaAccessControls<CameraAccessOffer>? get() = control?.consent

    fun get(context: Context): CameraBinding {
        val app = context.applicationContext
        binding?.let { return it }
        val value = RemoteMediaBindings.camera(app, {
            app.getSystemService(DisplayManager::class.java).getDisplay(Display.DEFAULT_DISPLAY)?.rotation ?: 0
        }, { control?.requestLegacy() }, service = { run ->
            val intent = Intent(app, CameraService::class.java).putExtra("camera_run", run)
            if (run != null) app.startForegroundService(intent) else app.stopService(intent)
        })
        binding = value
        val owner = CameraActivityOwner(value, MediaConsentPolicy<CameraAccessOffer>(value::hasPermission, prompts), prompts)
        control = owner
        val dispatcher = CoroutineScope(SupervisorJob() + Handler(Looper.getMainLooper()).asCoroutineDispatcher())
        scope = dispatcher
        val access = ConnectionCameraAccess(ConnectionOwner.get(app))
        observation = MediaConsentObservation<CameraAccessOffer>(access.offers, owner.consent, dispatcher)
        captureObservation = MediaConsumerObservation<CameraAccessOffer, CameraEndpoint, CameraCaptureTarget>(access.captures, owner.consent.state,
            value::hasPermission, value::useTarget, dispatcher, ::CameraCaptureTarget)
        return value
    }

    fun attach(launch: (LocalPermissionPrompt) -> Boolean): LocalPermissionAttachment =
        checkNotNull(control) { "Camera owner not initialized" }.attach(launch)

    fun detach(identity: LocalPermissionAttachment): Boolean = control?.detach(identity) == true

    fun permissionResult(prompt: LocalPermissionPrompt, granted: Boolean): Boolean = prompts.result(prompt, granted)

    fun ownsService(run: String?): Boolean = binding?.ownsBackground(run) == true

    fun serviceStopped(run: String?) {
        binding?.backgroundStopped(run)
    }

    fun reset() {
        captureObservation?.retire()
        captureObservation = null
        observation?.retire()
        observation = null
        scope?.cancel()
        scope = null
        control?.clear()
        binding?.shutdown()
        CameraInvitations.endpoint.value = null
        binding = null
        control = null
        prompts.clear()
    }
}
