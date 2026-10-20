package com.blent

import android.Manifest
import android.content.*
import android.os.*
import com.blent.connection.*
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraUsbCaptureOwnerTest {
    internal class NativeEndpoint : Binder() {
        val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
        val tasks = CopyOnWriteArrayList<MediaTask>()
        val commands = CopyOnWriteArrayList<Int>()
        @Volatile var alive = true
        var unlinked = 0
        override fun isBinderAlive() = alive
        override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { deaths.add(recipient) }
        override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int): Boolean {
            unlinked++; return deaths.remove(recipient)
        }
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            WorkerIpc.validate(data, WorkerIpc.REQUEST)
            data.readLong()
            commands.add(code)
            if (code == WorkerIpc.START) tasks.add(MediaTaskJson.decode(requireNotNull(data.readString())))
            return true
        }
        fun die() { alive = false; deaths.forEach { it.binderDied() } }
    }
    private class App(private val native: NativeEndpoint) : ContextWrapper(RuntimeEnvironment.getApplication()) {
        override fun getApplicationContext() = this
        override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
            assertEquals(CameraWorkerService::class.java.name, intent.component!!.className)
            connection.onServiceConnected(intent.component, native)
            return true
        }
        override fun unbindService(connection: ServiceConnection) {}
    }
    private suspend fun until(condition: () -> Boolean) = withTimeout(3000) {
        while (!condition()) {
            Shadows.shadowOf(Looper.getMainLooper()).idleFor(1, TimeUnit.MILLISECONDS)
            delay(1)
        }
    }

    @Test fun t932ProcessCompositionRequiresLocalStartAndActualNativeDeathBeforeProof() = runBlocking {
        CameraOwner.reset()
        val fixture = ConnectionUiFixture()
        val native = NativeEndpoint()
        Shadows.shadowOf(RuntimeEnvironment.getApplication()).grantPermissions(Manifest.permission.CAMERA)
        try {
            val binding = CameraOwner.get(App(native))
            val controls = checkNotNull(CameraOwner.access)
            CameraOwner.attach { error("T932 granted permission must not prompt") }
            fixture.connect(); fixture.prompt()
            val worker = fixture.workers.last()
            worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
            until { controls.state.value.offer != null }
            assertTrue(native.tasks.isEmpty())
            assertTrue(controls.start(controls.state.value.offer!!))
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
            until { native.tasks.isNotEmpty() }
            val task = native.tasks.single() as MediaTask.Camera
            assertEquals(ConnectionCameraTarget.endpoint(cameraRoute()), task.endpoint)
            assertEquals(CameraLens.REAR, task.lens)
            assertEquals(CameraLens.REAR, binding.selected)
            val admitted = controls.state.value.offer!!
            assertTrue(controls.stop(admitted))
            worker.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
            until { native.commands.contains(WorkerIpc.EXIT) }
            assertNull(binding.selected)
            assertTrue(fixture.binding.state.value.authenticated)
            assertFalse(worker.commands.any { it is ConnectionWorkerCommand.CameraConsumerRetired })
            assertEquals(0, native.unlinked)
            native.die()
            until { worker.commands.any { it is ConnectionWorkerCommand.CameraConsumerRetired } }
            assertEquals(1, native.unlinked)
            assertEquals(listOf(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())),
                worker.commands.filterIsInstance<ConnectionWorkerCommand.CameraConsumerRetired>())
        } finally {
            native.die(); CameraOwner.reset()
            Shadows.shadowOf(Looper.getMainLooper()).idleFor(10, TimeUnit.MILLISECONDS)
            fixture.close()
        }
    }
}
