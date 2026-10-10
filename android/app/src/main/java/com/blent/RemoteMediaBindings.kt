package com.blent

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import kotlinx.coroutines.*
import kotlinx.coroutines.android.asCoroutineDispatcher

/** Consent/UI/service policy stays in the main process; run ownership is remote. */
internal object RemoteMediaBindings {
    private fun scope() = CoroutineScope(SupervisorJob() + android.os.Handler(android.os.Looper.getMainLooper()).asCoroutineDispatcher())
    fun audio(context: Context, direction: Int, requestPermission: () -> Unit, service: (String?) -> Unit): AudioBinding =
        AudioBinding(AudioPreferencesStore.load(context, direction), requestPermission,
            { direction == 2 || context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED },
            { endpoint, settings, report -> AndroidWorkerRun(context, MediaTask.Audio(endpoint, settings), report = {
                if (it is WorkerEvent.Status) report(it.text)
            }) }, if (direction == 1) AudioInvitations.microphone else AudioInvitations.speakers,
            scope(), service, direction, { AudioPreferencesStore.save(it, context, direction) })

    fun camera(context: Context, rotation: () -> Int, requestPermission: () -> Unit, service: (String?) -> Unit): CameraBinding =
        CameraBinding(rotation, requestPermission, { context.checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED },
            { endpoint, lens, angle, report -> AndroidWorkerRun(context, MediaTask.Camera(endpoint, lens, angle()), report = {
                if (it is WorkerEvent.Status) report(it.text)
            }) }, CameraInvitations.endpoint, scope(), service)
}
