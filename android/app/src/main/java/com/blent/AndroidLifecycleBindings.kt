package com.blent

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import kotlinx.coroutines.*
import kotlinx.coroutines.android.asCoroutineDispatcher
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

internal class AudioResources {
    private val stopped = AtomicBoolean(false)
    private val socket = AtomicReference<Socket?>(null)
    fun attach(value: Socket?) {
        socket.set(value)
        if (stopped.get()) runCatching { socket.getAndSet(null)?.close() }
    }
    fun cancel() { stopped.set(true); runCatching { socket.getAndSet(null)?.close() } }
}

/** Android construction adapter; lifecycle contracts contain no Context or native handles. */
internal fun AudioBinding(context: Context,
    requestPermission: () -> Unit,
    permission: () -> Boolean = { context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED },
    capture: suspend (AudioEndpoint, AudioPreferences, (Socket?) -> Unit, (String) -> Unit) -> Unit = AudioCapture(context)::run,
    invitations: MutableStateFlow<AudioEndpoint?> = AudioInvitations.microphone,
    scope: CoroutineScope = CoroutineScope(SupervisorJob() + android.os.Handler(android.os.Looper.getMainLooper()).asCoroutineDispatcher()),
    service: (String?) -> Unit = {},
    direction: Int = 1,
): AudioBinding = AudioBinding(AudioPreferencesStore.load(context, direction), requestPermission, permission,
    { endpoint, settings, report ->
        val owned = AudioResources()
        object : LifecycleRun {
            override suspend fun execute() = capture(endpoint, settings, owned::attach, report)
            override fun cancel() = owned.cancel()
            override suspend fun retire() = owned.cancel()
        }
    }, invitations, scope, service, direction, { AudioPreferencesStore.save(it, context, direction) })

internal fun CameraBinding(context: Context,
    rotation: () -> Int,
    requestPermission: () -> Unit,
    permission: () -> Boolean = { context.checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED },
    capture: suspend (CameraEndpoint, CameraLens, Int, CameraResources) -> Unit = CameraCapture(context)::run,
    invitations: StateFlow<CameraEndpoint?> = CameraInvitations.endpoint,
    scope: CoroutineScope = CoroutineScope(SupervisorJob() + android.os.Handler(android.os.Looper.getMainLooper()).asCoroutineDispatcher()),
    backgroundService: (String?) -> Unit = {},
): CameraBinding = CameraBinding(rotation, requestPermission, permission, { endpoint, lens, angle ->
    val owned = CameraResources()
    object : LifecycleRun {
        override suspend fun execute() = capture(endpoint, lens, angle(), owned)
        override fun cancel() = owned.cancel()
        override suspend fun retire() = owned.close()
    }
}, invitations, scope, backgroundService)
