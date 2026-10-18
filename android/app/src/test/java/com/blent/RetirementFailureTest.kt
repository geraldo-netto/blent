package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import java.util.concurrent.atomic.AtomicInteger

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class RetirementFailureTest {
    @Test fun t807_nativeCloseFailureIsReportedWithoutRetryingUncertainRelease() {
        val resources = CameraResources()
        var successful = 0
        var attempts = 0
        resources.own { successful++ }
        resources.own { attempts++; error("native close failed") }
        val failure = runCatching { resources.close() }.exceptionOrNull()
        assertNotNull("T807 incomplete destruction reported success", failure)
        assertEquals(1, successful)
        assertEquals(1, attempts)
        assertSame("T807 unknown native state must remain quarantined", failure,
            runCatching { resources.close() }.exceptionOrNull())
        assertEquals("T807 uncertain destruction retried", 1, attempts)
    }

    @Test fun t807_audioServiceStopFailureStaysInsideOwner() = runBlocking {
        val errors = mutableListOf<Throwable>()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined + CoroutineExceptionHandler { _, e -> errors.add(e) })
        val finish = CompletableDeferred<Unit>()
        val commands = MutableStateFlow<AudioEndpoint?>(null)
        var runId: String? = null
        val binding = AudioBinding(RuntimeEnvironment.getApplication(), {}, { true }, { _, _, _, _ -> finish.await() },
            commands, scope, { run -> if (run == null) error("service stop failed") else runId = run })
        try {
            binding.configure(AudioPreferences(background = true))
            binding.start()
            commands.value = AudioEndpoint("a".repeat(64), 1234, 1, 1, 20, true)
            val worker = scope.coroutineContext[Job]!!.children.last()
            finish.complete(Unit)
            withTimeout(2000) { worker.join() }
            assertTrue("T807 cleanup escaped root coroutine: $errors", errors.isEmpty())
            assertFalse(binding.sharing)
            assertTrue(binding.status.contains("service stop failed"))
            assertTrue("T807 unconfirmed service ownership discarded", binding.ownsBackground(runId))
            assertTrue(runCatching { binding.stopSharing() }.isSuccess)
        } finally { runCatching { binding.shutdown() }; scope.cancel() }
    }

    @Test fun t807_cameraServiceStopFailureStillRetiresNativeOwner() = runBlocking {
        val errors = mutableListOf<Throwable>()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined + CoroutineExceptionHandler { _, e -> errors.add(e) })
        val finish = CompletableDeferred<Unit>()
        val commands = MutableStateFlow<CameraEndpoint?>(null)
        val closed = AtomicInteger()
        val binding = CameraBinding(RuntimeEnvironment.getApplication(), { 0 }, {}, { true },
            { _, _, _, owned -> owned.own { closed.incrementAndGet() }; finish.await() }, commands, scope,
            { run -> if (run == null) error("service stop failed") })
        try {
            binding.start()
            commands.value = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000, CameraLens.FRONT, true)
            val worker = scope.coroutineContext[Job]!!.children.last()
            finish.complete(Unit)
            withTimeout(2000) { worker.join() }
            assertTrue("T807 cleanup escaped root coroutine: $errors", errors.isEmpty())
            assertEquals(1, closed.get())
            assertNull(binding.selected)
            assertTrue(binding.status.contains("service stop failed"))
            assertTrue(runCatching { binding.shutdown() }.isSuccess)
        } finally { runCatching { binding.shutdown() }; scope.cancel() }
    }

    @Test fun t807_activityStopContinuesAfterIndependentServiceFailure() {
        val app = RuntimeEnvironment.getApplication()
        Prefs(app).checkUpdates = false
        val controller = Robolectric.buildActivity(MainActivity::class.java).create()
        val activity = controller.get()
        fun field(target: Any, name: String) = target.javaClass.getDeclaredField(name).apply { isAccessible = true }
        field(activity.session, "touchCapture").set(activity.session, null)
        field(activity.session, "videoReceiver").set(activity.session, null)
        val rig = PowerRuntimeFixture()
        var failures = 0
        val port = object : PowerServicePort {
            override fun post(notice: DisplayPowerNotice): Boolean { rig.notices.add(notice); return true }
            override fun stop() { failures++; error("T807 actual power service stop failed") }
        }
        check(rig.runtime.attach(port))
        val broken = StreamingPowerBinding(app, rig.runtime)
        var previous: StreamingPowerBinding? = null
        try {
            controller.start()
            val original = field(activity, "powerBinding").get(activity) as StreamingPowerBinding
            previous = original
            val observed = field(original, "scope").get(original) as CoroutineScope
            val originalJob = checkNotNull(observed.coroutineContext[Job])
            original.stop()
            org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
            check(originalJob.isCompleted) { "T1064 old observer must settle before fixture replacement" }
            broken.start(DisplayPowerDemand(StreamingPower()), MutableStateFlow(DisplayPowerDemand(StreamingPower())))
            field(activity, "powerBinding").set(activity, broken)
            controller.stop()
            org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
            assertTrue("T1064 original foreground observer leaked across fixture handoff", originalJob.isCompleted)
            assertEquals("T807 fixture did not exercise actual stop failure", 1, failures)
            assertEquals("T807 one failure skipped session retirement", DisplayWakeState.BACKGROUND, activity.session.powerNow().display)
        } finally {
            previous?.stop(); controller.destroy()
            rig.runtime.detach(port)
            while (true) rig.runtime.canceled(rig.notices.poll() ?: break)
            rig.scope.cancel()
        }
    }
}
