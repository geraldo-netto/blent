package com.blent

import android.content.Context
import android.content.Intent
import android.hardware.display.DisplayManager
import android.view.Display

/** Process camera owner holds only the application; Activity callbacks detach onStop. */
internal object CameraOwner {
    private var binding: CameraBinding? = null
    private var control: CameraActivityOwner? = null
    private val prompts = CameraPermissionPrompts()

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
        control = CameraActivityOwner(value, CameraConsentPolicy(value::hasPermission, prompts), prompts)
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
        control?.clear()
        binding?.shutdown()
        CameraInvitations.endpoint.value = null
        binding = null
        control = null
        prompts.clear()
    }
}
