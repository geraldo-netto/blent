package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.atomic.AtomicInteger

/** T810: ordinary JVM tests use resource contracts, without Android or Compose. */
class PortableLifecycleTest {
    private class Run : LifecycleRun {
        val opened = AtomicInteger()
        val cancelled = AtomicInteger()
        val retired = AtomicInteger()
        override suspend fun execute() { opened.incrementAndGet(); awaitCancellation() }
        override fun cancel() { cancelled.incrementAndGet() }
        override suspend fun retire() { retired.incrementAndGet() }
    }

    @Test fun t810_audioPolicyPublishesStateAndDelegatesOwnedResources() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val invitations = MutableStateFlow<AudioEndpoint?>(null)
        val run = Run()
        val saved = mutableListOf<AudioPreferences>()
        val reports = mutableListOf<(String) -> Unit>()
        val binding = AudioBinding(AudioPreferences(), {}, { true }, { _, _, report -> reports.add(report); run },
            invitations, scope, {}, 1, saved::add)
        try {
            binding.start()
            invitations.value = AudioEndpoint("a".repeat(64), 1234, 1, 1, 20, false)
            assertEquals(1, run.opened.get())
            assertTrue(binding.state.value.sharing)
            reports.single()("ready")
            assertEquals("ready", binding.state.value.status)
            binding.stopSharing()
            assertFalse(binding.state.value.sharing)
            assertTrue(run.cancelled.get() > 0)
            assertEquals(1, run.retired.get())
            reports.single()("obsolete")
            assertNotEquals("obsolete", binding.state.value.status)
            val value = AudioPreferences(gain = 120)
            binding.configure(value)
            assertEquals(listOf(value), saved)
            assertEquals(value, binding.state.value.preferences)
        } finally { binding.shutdown(); scope.cancel() }
    }

    @Test fun t810_cameraPolicyRetainsRunUntilRetirementAndObservesRotationAtExecution() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val invitations = MutableStateFlow<CameraEndpoint?>(null)
        val run = Run()
        var rotation = 1
        var readRotation: (() -> Int)? = null
        val binding = CameraBinding({ rotation }, {}, { true }, { _, _, angle -> readRotation = angle; run },
            invitations, scope, {})
        try {
            binding.start()
            invitations.value = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000, CameraLens.FRONT)
            assertEquals(CameraLens.FRONT, binding.state.value.selected)
            assertEquals(1, run.opened.get())
            rotation = 2
            assertEquals(2, readRotation!!())
            binding.choose(null)
            assertNull(binding.state.value.selected)
            withTimeout(2000) { while (run.retired.get() == 0) delay(1) }
            assertTrue(run.cancelled.get() > 0)
        } finally { binding.shutdown(); scope.cancel() }
    }
}
