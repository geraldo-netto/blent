package com.blent

import android.content.*
import android.os.*
import android.graphics.SurfaceTexture
import android.view.*
import kotlinx.coroutines.*
import kotlinx.coroutines.android.asCoroutineDispatcher
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
        val commands = java.util.concurrent.CopyOnWriteArrayList<Int>()
        @Volatile var unlinked = 0
        @Volatile var alive = true
        var exitWorks = true
        override fun isBinderAlive() = alive
        override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { deaths.add(recipient) }
        override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int): Boolean { unlinked++; return deaths.remove(recipient) }
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            WorkerIpc.validate(data, WorkerIpc.REQUEST)
            val generation = data.readLong()
            commands.add(code)
            when (code) {
                WorkerIpc.START -> {
                    MediaTaskJson.decode(requireNotNull(data.readString()))
                    val callback = requireNotNull(data.readStrongBinder())
                    WorkerIpc.event(callback, generation, WorkerEventJson.encode(WorkerEvent.Result("ignored")))
                    WorkerIpc.event(callback, generation, WorkerEventJson.encode(WorkerEvent.Status("remote ready")))
                    started.complete(Unit)
                }
                WorkerIpc.EXIT -> if (exitWorks) { alive = false; deaths.toList().forEach { it.binderDied() } }
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
        while (!condition()) { Shadows.shadowOf(Looper.getMainLooper()).idleFor(1, java.util.concurrent.TimeUnit.MILLISECONDS); delay(1) }
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
        } finally {
            audio.shutdown()
            try { until { audioEndpoint.deaths.isEmpty() } }
            catch (failure: TimeoutCancellationException) {
                throw AssertionError("T962 audio status=${audio.status}; commands=${audioEndpoint.commands}; unlinked=${audioEndpoint.unlinked}", failure)
            }
        }
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
    @Test fun t962PausedLooperRetiresOnlyAfterActualDeathAndDrivesFallbackTimer() = runBlocking {
        val endpoint = Endpoint().apply { exitWorks = false }
        val scope = CoroutineScope(SupervisorJob() + Handler(Looper.getMainLooper()).asCoroutineDispatcher())
        var proof: ResourceRetirement? = null
        val lane = ResourceLane(scope, {})
        val run = AndroidWorkerRun(App(endpoint), MediaTask.Metadata(640, 480, 30), graceMillis = 1, exitMillis = 3000)
        try {
            lane.replace({ run }, {}, { proof = it })
            until { endpoint.started.isCompleted }
            lane.stop(); scope.cancel()
            until { endpoint.commands.contains(WorkerIpc.EXIT) }
            // Advancing the fixture clock is not process death or native proof.
            repeat(8) { Shadows.shadowOf(Looper.getMainLooper()).idleFor(1, java.util.concurrent.TimeUnit.MILLISECONDS); delay(1) }
            assertTrue(endpoint.alive)
            assertNull(proof); assertEquals(0, endpoint.unlinked)
            endpoint.alive = false // Exact simulated native process death; no callback.
            until { endpoint.deaths.isEmpty() }
            until { proof != null }
            assertEquals(ResourceRetirement.RETIRED, proof)
            assertEquals(1, endpoint.unlinked)
        } finally {
            endpoint.alive = false
            run.cancel(); scope.cancel()
            // Release the owned fixture even on the intentionally red old waiter.
            withTimeout(3000) {
                while (!scope.coroutineContext[Job]!!.isCompleted) {
                    Shadows.shadowOf(Looper.getMainLooper()).idleFor(5, java.util.concurrent.TimeUnit.MILLISECONDS)
                    delay(1)
                }
            }
        }
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
