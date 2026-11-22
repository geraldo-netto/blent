package com.blent

import android.Manifest
import android.content.*
import android.hardware.display.DisplayManager
import android.os.Looper
import android.view.Display
import kotlinx.coroutines.*
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraDisplayRotationTest {
    @Test fun t1572NativeDisplayRegistrationFencesOldCallbacksAndUsesDefaultDisplay() {
        val app = RuntimeEnvironment.getApplication()
        val display = app.getSystemService(DisplayManager::class.java).getDisplay(Display.DEFAULT_DISPLAY)
        val source = AndroidDisplayRotation(app)
        var changes = 0
        source.start { changes++ }
        val first = ReflectionHelpers.getField<DisplayManager.DisplayListener>(source, "owner")
        try {
            assertThrows(IllegalStateException::class.java) { source.start {} }
            first.onDisplayChanged(42); first.onDisplayAdded(42); first.onDisplayRemoved(0)
            assertEquals(0, changes)
            first.onDisplayAdded(0)
            assertEquals(1, changes)
            Shadows.shadowOf(display).setRotation(3)
            Shadows.shadowOf(Looper.getMainLooper()).idle()
            assertEquals(3, source.current())
            assertTrue("T1572 registered callback did not receive display change", changes > 1)
            source.stop(); source.stop()
            val previous = changes
            first.onDisplayChanged(0)
            source.start { changes += 10 }
            first.onDisplayChanged(0)
            assertEquals(previous, changes)
            Shadows.shadowOf(display).setRotation(1)
            Shadows.shadowOf(Looper.getMainLooper()).idle()
            assertTrue(changes >= previous + 10)
            assertEquals(1, source.current())
        } finally { source.stop() }
    }

    private class App : ContextWrapper(RuntimeEnvironment.getApplication()) {
        val workers = CopyOnWriteArrayList<CameraUsbCaptureOwnerTest.NativeEndpoint>()
        override fun getApplicationContext() = this
        override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
            assertEquals(CameraWorkerService::class.java.name, intent.component!!.className)
            val worker = CameraUsbCaptureOwnerTest.NativeEndpoint()
            workers.add(worker)
            connection.onServiceConnected(intent.component, worker)
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

    @Test fun t1572ProcessCameraOwnerReplacesWorkerWithCurrentDisplayRotation() = runBlocking {
        CameraOwner.reset()
        val app = App()
        Shadows.shadowOf(RuntimeEnvironment.getApplication()).grantPermissions(Manifest.permission.CAMERA)
        val display = app.getSystemService(DisplayManager::class.java).getDisplay(Display.DEFAULT_DISPLAY)
        Shadows.shadowOf(display).setRotation(0)
        val binding = CameraOwner.get(app)
        try {
            binding.start()
            CameraInvitations.endpoint.value = CameraEndpoint("a".repeat(64), 12345, 640, 480, 30, 3000, CameraLens.FRONT)
            until { app.workers.firstOrNull()?.tasks?.isNotEmpty() == true }
            val first = app.workers.single()
            assertEquals(0, (first.tasks.single() as MediaTask.Camera).rotation)
            Shadows.shadowOf(display).setRotation(3)
            until { first.commands.contains(WorkerIpc.EXIT) }
            assertEquals("T1572 replacement preceded original native death", 1, app.workers.size)
            first.die()
            until { app.workers.size == 2 && app.workers.last().tasks.isNotEmpty() }
            assertEquals(3, (app.workers.last().tasks.single() as MediaTask.Camera).rotation)
            assertEquals(1, first.unlinked)
            assertEquals(CameraLens.FRONT, binding.selected)
        } finally {
            binding.choose(null)
            app.workers.forEach { it.die() }
            CameraOwner.reset()
            Shadows.shadowOf(Looper.getMainLooper()).idleFor(10, TimeUnit.MILLISECONDS)
        }
    }
}
