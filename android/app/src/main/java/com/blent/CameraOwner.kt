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
    private val prompts = CameraPermissionPrompts()
    private var observation: CameraConsentObservation? = null
    private var captureObservation: CameraCaptureObservation? = null
    private var scope: CoroutineScope? = null
    val access: CameraAccessControls? get() = control?.consent

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
        val owner = CameraActivityOwner(value, CameraConsentPolicy(value::hasPermission, prompts), prompts)
        control = owner
        val dispatcher = CoroutineScope(SupervisorJob() + Handler(Looper.getMainLooper()).asCoroutineDispatcher())
        scope = dispatcher
        val access = ConnectionCameraAccess(ConnectionOwner.get(app))
        observation = CameraConsentObservation(access.offers, owner.consent, dispatcher)
        captureObservation = CameraCaptureObservation(access.captures, owner.consent.state,
            value::hasPermission, value::useTarget, dispatcher)
        return value
    }

    fun attach(launch: (CameraPermissionPrompt) -> Boolean): CameraPermissionAttachment =
        checkNotNull(control) { "Camera owner not initialized" }.attach(launch)

    fun detach(identity: CameraPermissionAttachment): Boolean = control?.detach(identity) == true

    fun permissionResult(prompt: CameraPermissionPrompt, granted: Boolean): Boolean = prompts.result(prompt, granted)

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
