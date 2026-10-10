package com.blent

import android.content.*
import android.os.*
import android.graphics.SurfaceTexture
import android.view.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers
import java.lang.reflect.Proxy

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class RemoteAdaptersTest {
    private class Endpoint : Binder() {
        val deaths = mutableListOf<IBinder.DeathRecipient>()
        val started = CompletableDeferred<Unit>()
        override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { deaths.add(recipient) }
        override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int) = deaths.remove(recipient)
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            WorkerIpc.validate(data, WorkerIpc.REQUEST)
            val generation = data.readLong()
            when (code) {
                WorkerIpc.START -> {
                    MediaTaskJson.decode(requireNotNull(data.readString()))
                    val callback = requireNotNull(data.readStrongBinder())
                    WorkerIpc.event(callback, generation, WorkerEventJson.encode(WorkerEvent.Result("ignored")))
                    WorkerIpc.event(callback, generation, WorkerEventJson.encode(WorkerEvent.Status("remote ready")))
                    started.complete(Unit)
                }
                WorkerIpc.EXIT -> deaths.toList().forEach { it.binderDied() }
            }
            return true
        }
    }
    private class App(val endpoint: Endpoint) : ContextWrapper(RuntimeEnvironment.getApplication()) {
        override fun getApplicationContext() = this
        override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
            connection.onServiceConnected(intent.component, endpoint); return true
        }
        override fun unbindService(connection: ServiceConnection) {}
    }
    private suspend fun until(condition: () -> Boolean) = withTimeout(3000) {
        while (!condition()) { Shadows.shadowOf(Looper.getMainLooper()).idle(); delay(1) }
    }
    @Test fun t809_remoteAudioAndCameraStatusReachConsentOwners() = runBlocking {
        val app = RuntimeEnvironment.getApplication()
        Shadows.shadowOf(app).grantPermissions(android.Manifest.permission.CAMERA)
        val audioEndpoint = Endpoint()
        val audio = RemoteMediaBindings.audio(App(audioEndpoint), 2, {}, {})
        try {
            audio.start(); Shadows.shadowOf(Looper.getMainLooper()).idle()
            AudioInvitations.speakers.value = AudioEndpoint("a".repeat(64), 1234, 2, 1, 40, false)
            until { audio.status == "remote ready" }
            assertTrue(audio.sharing)
        } finally { audio.shutdown(); until { audioEndpoint.deaths.isEmpty() } }
        val cameraEndpoint = Endpoint()
        val camera = RemoteMediaBindings.camera(App(cameraEndpoint), { 0 }, {}, {})
        try {
            CameraInvitations.endpoint.value = null
            camera.start(); Shadows.shadowOf(Looper.getMainLooper()).idle()
            CameraInvitations.endpoint.value = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000, CameraLens.FRONT)
            until { camera.status == "remote ready" }
            assertEquals(CameraLens.FRONT, camera.selected)
        } finally { camera.shutdown(); until { cameraEndpoint.deaths.isEmpty() }; CameraInvitations.endpoint.value = null }
    }
    @Test fun t809_defaultRemoteDisplayFactoryUsesBinderAndReleasesIt() = runBlocking {
        val endpoint = Endpoint()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val surface = Surface(SurfaceTexture(0))
        val view = object : SurfaceView(RuntimeEnvironment.getApplication()) {
            override fun getHolder(): SurfaceHolder = Proxy.newProxyInstance(SurfaceHolder::class.java.classLoader,
                arrayOf(SurfaceHolder::class.java)) { _, method, _ -> if (method.name == "getSurface") surface else null } as SurfaceHolder
        }
        val receiver = RemoteDisplay(App(endpoint), scope)
        try {
            receiver.token = "a".repeat(64); receiver.setSurface(view); receiver.start()
            withTimeout(2000) { endpoint.started.await() }
            assertTrue(endpoint.deaths.isNotEmpty())
            receiver.stop(); until { endpoint.deaths.isEmpty() }
            assertTrue(surface.isValid)
        } finally { receiver.stop(); scope.cancel(); surface.release() }
    }
    @Test fun t809_remoteMetadataRoundTripAndUnavailableFallbackAreExplicit() = runBlocking {
        WorkerTestBridge.install()
        val app = RuntimeEnvironment.getApplication()
        assertEquals(640, RemoteMetadata.report(app, 640, 480, 30).getInt("width"))
        val owner = ReflectionHelpers.getStaticField<WorkerQueries>(RemoteMetadata::class.java, "owner")
        until { !owner.busy }
        ReflectionHelpers.setStaticField(RemoteMetadata::class.java, "owner", WorkerQueries({ _, _ -> error("unavailable") }))
        try {
            val missing = RemoteMetadata.report(app, 1280, 720, 60)
            assertEquals("unavailable", missing.getString("status"))
            assertEquals(0, missing.getJSONArray("hardware").length())
        } finally { ReflectionHelpers.setStaticField(RemoteMetadata::class.java, "owner", null) }
    }
}
