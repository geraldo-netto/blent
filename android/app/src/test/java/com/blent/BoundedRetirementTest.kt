package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

/** T806: request storms must not construct a chain of native owners or jobs. */
class BoundedRetirementTest {
    private class Runs {
        val release = CompletableDeferred<Unit>()
        val created = AtomicInteger()
        val opened = AtomicInteger()
        val retired = AtomicInteger()
        fun create(): LifecycleRun {
            created.incrementAndGet()
            return object : LifecycleRun {
                private val done = AtomicBoolean()
                override suspend fun execute() { opened.incrementAndGet(); awaitCancellation() }
                override fun cancel() {}
                override suspend fun retire() {
                    release.await()
                    if (done.compareAndSet(false, true)) retired.incrementAndGet()
                }
            }
        }
    }

    @Test fun t806_audioReplacementStormRetainsOneOwnerAndStopDiscardsPendingRequest() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val commands = MutableStateFlow<AudioEndpoint?>(null)
        val runs = Runs()
        val binding = AudioBinding(AudioPreferences(), {}, { true }, { _, _, _ -> runs.create() }, commands, scope, {}, 1, {})
        try {
            binding.start()
            val request = AudioEndpoint("a".repeat(64), 1234, 1, 1, 20, false)
            commands.value = request
            repeat(128) { commands.value = request.copy(port = 1235 + it) }
            assertEquals("T806 queued requests constructed resource owners", 1, runs.created.get())
            assertTrue("T806 cancelled requests retained job chain", scope.coroutineContext[Job]!!.children.count() <= 4)
            assertEquals(1, runs.opened.get())
            binding.stopSharing()
            runs.release.complete(Unit)
            withTimeout(2000) { while (runs.retired.get() == 0) delay(1) }
            assertEquals("T806 Stop admitted pending capture", 1, runs.opened.get())
        } finally {
            runs.release.complete(Unit); binding.shutdown(); scope.cancel()
            withTimeout(2000) { scope.coroutineContext[Job]!!.join() }
        }
    }

    @Test fun t806_cameraReplacementStormAdmitsOnlyLatestAfterRetirement() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val commands = MutableStateFlow<CameraEndpoint?>(null)
        val runs = Runs()
        val admitted = mutableListOf<CameraLens>()
        val binding = CameraBinding({ 0 }, {}, { true }, { _, lens, _, _ -> admitted.add(lens); runs.create() }, commands, scope, {})
        try {
            binding.start()
            commands.value = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000, CameraLens.FRONT)
            repeat(128) { binding.choose(CameraLens.FRONT) }
            binding.choose(CameraLens.REAR)
            assertEquals("T806 pending replacements allocated owners", 1, runs.created.get())
            assertEquals(1, runs.opened.get())
            runs.release.complete(Unit)
            withTimeout(2000) { while (runs.opened.get() != 2) delay(1) }
            assertEquals(listOf(CameraLens.FRONT, CameraLens.REAR), admitted)
            assertEquals(1, runs.retired.get())
        } finally {
            runs.release.complete(Unit); binding.shutdown(); scope.cancel()
            withTimeout(2000) { scope.coroutineContext[Job]!!.join() }
        }
    }
    @Test fun t806_cameraStopCannotHideAnAlreadyStalledNativeOwner() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val commands = MutableStateFlow<CameraEndpoint?>(null)
        val runs = Runs()
        val binding = CameraBinding({ 0 }, {}, { true }, { _, _, _, _ -> runs.create() }, commands, scope, {})
        try {
            binding.start()
            commands.value = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000, CameraLens.FRONT)
            binding.choose(CameraLens.REAR)
            withTimeout(7000) { while (!binding.status.contains("stalled")) delay(1) }
            binding.choose(null)
            assertTrue("T806 Stop hid stalled ownership", binding.status.contains("stalled"))
            binding.shutdown()
            assertTrue("T806 shutdown hid stalled ownership", binding.status.contains("stalled"))
            assertEquals(1, runs.created.get())
        } finally {
            runs.release.complete(Unit); binding.shutdown(); scope.cancel()
            withTimeout(2000) { scope.coroutineContext[Job]!!.join() }
        }
    }

    @Test fun t806_cameraRetirementFailureRejectsLaterFactories() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val commands = MutableStateFlow<CameraEndpoint?>(null)
        var created = 0
        val binding = CameraBinding({ 0 }, {}, { true }, { _, _, _, _ ->
            created++
            object : LifecycleRun {
                override suspend fun execute() { awaitCancellation() }
                override fun cancel() {}
                override suspend fun retire() { error("uncertain release") }
            }
        }, commands, scope, {})
        try {
            binding.start()
            commands.value = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000, CameraLens.FRONT)
            binding.choose(CameraLens.REAR)
            assertEquals(1, created)
            assertTrue(binding.status.contains("uncertain release"))
            assertNull(binding.selected)
        } finally { binding.shutdown(); scope.cancel() }
    }

}
