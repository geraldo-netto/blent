package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import kotlin.coroutines.CoroutineContext

class StatusDispatchTest {
    private class Queued : CoroutineDispatcher() {
        val pending = ArrayDeque<Pair<CoroutineContext, Runnable>>()
        override fun dispatch(context: CoroutineContext, block: Runnable) { pending.add(context to block) }
        fun cancelFirst() { pending.first().first[Job]!!.cancel() }
        fun drain() { var budget = 30000; while (pending.isNotEmpty()) { check(budget-- > 0); pending.removeFirst().second.run() } }
    }
    @Test fun t830_audioStatusBurstRetainsOnePendingDispatchAndLatestGeneration() {
        val dispatcher = Queued()
        val scope = CoroutineScope(SupervisorJob() + dispatcher)
        val invitations = MutableStateFlow<AudioEndpoint?>(null)
        lateinit var report: (String) -> Unit
        val binding = AudioBinding(AudioPreferences(), {}, { true }, { _, _, status ->
            report = status; heldRun()
        }, invitations, scope, {}, 1, {})
        try {
            binding.start(); dispatcher.drain()
            invitations.value = AudioEndpoint("a".repeat(64), 1234, 1, 1, 40, false); dispatcher.drain()
            repeat(10000) { report("status $it") }
            assertEquals("T830 status jobs grow with producer count", 1, dispatcher.pending.size)
            dispatcher.drain(); assertEquals("status 9999", binding.status)
            injectFailure(binding); dispatcher.drain()
            assertEquals("Status update failed: injected", binding.status)
            report("stale"); binding.stopSharing(); dispatcher.drain()
            assertNotEquals("stale", binding.status)
        } finally { binding.shutdown(); dispatcher.drain() }
    }
    @Test fun t830_cameraStatusBurstRetainsOnePendingDispatchAndLatestGeneration() {
        val dispatcher = Queued()
        val scope = CoroutineScope(SupervisorJob() + dispatcher)
        val invitations = MutableStateFlow<CameraEndpoint?>(null)
        lateinit var report: (String) -> Unit
        val binding = CameraBinding({ 0 }, {}, { true }, { _, _, _, status -> report = status; heldRun() }, invitations, scope, {})
        try {
            binding.start(); dispatcher.drain()
            invitations.value = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000, requestedLens = CameraLens.REAR); dispatcher.drain()
            repeat(10000) { report("status $it") }
            assertEquals("T830 camera status jobs grow with producer count", 1, dispatcher.pending.size)
            dispatcher.drain(); assertEquals("status 9999", binding.status)
            injectFailure(binding); dispatcher.drain()
            assertEquals("Status update failed: injected", binding.status)
            report("stale"); binding.choose(null); dispatcher.drain()
            assertNotEquals("stale", binding.status)
        } finally { binding.shutdown(); scope.cancel(); dispatcher.drain() }
    }
    @Test fun t830_dispatchFailureCancellationAndReentrantUpdatesStayBounded() {
        val dispatcher = Queued()
        val scope = CoroutineScope(SupervisorJob() + dispatcher)
        val errors = mutableListOf<Exception>()
        val latest = LatestDispatch(scope) { errors.add(it); error("report failed") }
        latest.submit { error("update failed") }; dispatcher.drain()
        assertEquals("update failed", latest.failure?.message)
        assertEquals("report failed", latest.failure!!.suppressed.single().message)
        assertEquals(1, errors.size)
        var observed = 0
        latest.submit { observed = 1; latest.submit { observed = 2 } }; dispatcher.drain()
        assertEquals(2, observed)
        latest.submit { observed = 3 }; scope.cancel(); dispatcher.drain()
        assertEquals(2, observed)
        latest.submit { observed = 4 }; dispatcher.drain(); assertEquals(2, observed)
        val direct = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val immediate = LatestDispatch(direct) { throw it }
        immediate.submit { observed = 5 }; assertEquals(5, observed)
        immediate.submit { error("same exception") }
        assertEquals(0, immediate.failure!!.suppressed.size)
        direct.cancel()
    }

    @Test fun t1263CancelledDrainCannotDiscardNewerObservation() {
        for (count in listOf(1, 2, 1000)) {
            val dispatcher = Queued()
            val scope = CoroutineScope(SupervisorJob() + dispatcher)
            val latest = LatestDispatch(scope) { throw it }
            var observed = -1
            try {
                latest.submit { error("T1263 rejected original action ran") }
                dispatcher.cancelFirst()
                repeat(count) { value -> latest.submit { observed = value } }
                assertEquals("T1263 more than one pending drain", 1, dispatcher.pending.size)
                dispatcher.drain()
                assertEquals("T1263 newer observation lost during cancelled drain cleanup", count - 1, observed)
                assertTrue(dispatcher.pending.isEmpty())
            } finally { scope.cancel(); dispatcher.drain() }
        }
    }

    @Test fun t1263ClearAndScopeCancellationCutOffQueuedReplacement() {
        val dispatcher = Queued()
        val scope = CoroutineScope(SupervisorJob() + dispatcher)
        val latest = LatestDispatch(scope) { throw it }
        try {
            latest.submit { error("T1263 cancelled original") }; dispatcher.cancelFirst()
            latest.submit { error("T1263 cleared replacement") }; latest.clear(); dispatcher.drain()
            latest.submit { error("T1263 stopped owner") }; scope.cancel(); dispatcher.drain()
            assertTrue(dispatcher.pending.isEmpty())
        } finally { scope.cancel(); dispatcher.drain() }
    }

    private fun injectFailure(binding: Any) {
        val field = binding.javaClass.getDeclaredField("statusUpdates").apply { isAccessible = true }
        (field.get(binding) as LatestDispatch).submit { error("injected") }
    }
    private fun heldRun() = object : LifecycleRun {
        override suspend fun execute() = awaitCancellation()
        override fun cancel() {}
        override suspend fun retire() {}
    }
}
