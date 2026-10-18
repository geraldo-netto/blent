package com.blent

import android.content.ComponentName
import android.content.Intent
import android.os.Looper
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RuntimeEnvironment
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.android.controller.ServiceController
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [PowerTestWifiManager::class, PowerTestWifiLock::class])
@LooperMode(LooperMode.Mode.PAUSED)
class PowerWorkerIntegrationTest {
    private class Rig {
        val app = PowerRuntimeFixture.App()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val ui = ConcurrentLinkedQueue<() -> Unit>()
        val tasks = CopyOnWriteArrayList<MediaTask>()
        val ready = CopyOnWriteArrayList<MediaTask>()
        val retiring = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val runtime = StreamingPowerRuntime(app, postUi = { ui.add(it) }, scope = scope)
        val controller = ServiceController.of(StreamingService(runtime), Intent()).create()
        init {
            WorkerTestBridge.install()
            val bridge = WorkerTestBridge { task, surface, report -> create(task, surface, report) }
            shadowOf(RuntimeEnvironment.getApplication()).setComponentNameAndServiceForBindServiceForIntent(
                Intent(app, PowerWorkerService::class.java), ComponentName(app, PowerWorkerService::class.java), bridge)
        }
        private fun create(task: MediaTask, surface: android.view.Surface?, report: (WorkerEvent) -> Unit): LifecycleRun {
            tasks.add(task)
            val first = tasks.size == 1
            val native = LocalMediaRuns.create(app, task, surface) { event ->
                report(event)
                if (event is WorkerEvent.WakeReady) ready.add(task)
            }
            return object : LifecycleRun {
                override suspend fun execute() = native.execute()
                override fun cancel() = native.cancel()
                override suspend fun retire() {
                    if (first) { retiring.complete(Unit); release.await() }
                    native.retire()
                }
            }
        }
        suspend fun until(predicate: () -> Boolean) = withTimeout(5000) {
            while (!predicate()) {
                shadowOf(Looper.getMainLooper()).idle()
                while (true) (ui.poll() ?: break).invoke()
                delay(1)
            }
        }
        suspend fun close(owner: DisplayPowerOwner) {
            release.complete(Unit); runtime.close(owner); controller.destroy()
            try { until { scope.coroutineContext[Job]!!.children.none() } }
            finally { scope.cancel() }
        }
    }
    @Test fun t1060DefaultPrivateWorkerRetainsOriginalNativeCleanupBeforeReplacement() = runBlocking {
        val rig = Rig()
        val owner = rig.runtime.open(StreamingPower())
        try {
            rig.until { rig.ready.size == 1 }
            val originalCpu = org.robolectric.shadows.ShadowPowerManager.getLatestWakeLock()
            assertTrue(originalCpu.isHeld)
            rig.runtime.publish(owner, StreamingPower(display = DisplayWakeState.PAUSED))
            rig.until { rig.retiring.isCompleted }
            assertTrue("T1060 fixture must retain original native grant until actual cleanup", originalCpu.isHeld)
            rig.runtime.publish(owner, StreamingPower(true, true))
            repeat(8) { shadowOf(Looper.getMainLooper()).idle(); delay(1) }
            assertEquals("T1060 replacement bypassed actual original local/native fence", 1, rig.tasks.size)
            rig.release.complete(Unit)
            rig.until { rig.ready.size == 2 }
            assertFalse(originalCpu.isHeld)
            assertEquals(listOf(MediaTask.Wake(true, true), MediaTask.Wake(false, true)), rig.tasks)
        } finally { rig.close(owner) }
    }
}
