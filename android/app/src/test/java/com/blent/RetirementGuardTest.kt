package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class RetirementGuardTest {
    @Test fun t807_retirementFailureClosesAdmissionAndSuccessfulRetirementKeepsItOpen() = runBlocking {
        val guard = RetirementGuard()
        var failure: Exception? = null
        val run = object : LifecycleRun {
            override suspend fun execute() {}
            override fun cancel() {}
            override suspend fun retire() { failure?.let { throw it } }
        }
        guard.checkAvailable()
        assertNull(guard.retire(run))
        assertNull(guard.error)
        failure = IllegalStateException("native close failed")
        assertSame(failure, guard.retire(run))
        assertSame(failure, guard.error)
        assertThrows(IllegalStateException::class.java) { guard.checkAvailable() }
        val original = failure
        failure = IllegalArgumentException("another failed close")
        assertSame(original, guard.retire(run))
        assertSame(original, guard.error)
    }

    @Test fun t807_independentCleanupAttemptsAllActionsButDoesNotConsumeFatalErrors() {
        val attempted = mutableListOf<Int>()
        val first = IllegalStateException("first")
        val second = IllegalArgumentException("second")
        val errors = cleanupAll({ attempted.add(1); throw first }, { attempted.add(2) }, { attempted.add(3); throw second })
        assertEquals(listOf(1, 2, 3), attempted)
        assertEquals(listOf(first, second), errors)
        assertTrue(cleanupAll().isEmpty())
        assertThrows(AssertionError::class.java) { cleanupAll({ throw AssertionError("fatal") }, { fail("unreachable") }) }
    }

    @Test fun t807_lateAndInterruptCleanupFailuresRemainVisible() {
        val resources = CameraResources()
        val failure = IllegalStateException("interruption")
        resources.interrupt { throw failure }
        resources.own { throw failure }
        resources.own { error("another resource") }
        resources.cancel()
        assertSame(failure, runCatching { resources.close() }.exceptionOrNull())
        assertEquals(1, failure.suppressed.size)
        assertTrue(runCatching { resources.own { error("late resource") } }.isFailure)
        assertSame(failure, runCatching { resources.close() }.exceptionOrNull())
        assertEquals(2, failure.suppressed.size)
    }
    @Test fun t807_audioCannotReplaceAnUncertainNativeOwner() = runBlocking {
        val errors = mutableListOf<Throwable>()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined + CoroutineExceptionHandler { _, e -> errors.add(e) })
        val commands = MutableStateFlow<AudioEndpoint?>(null)
        val finish = CompletableDeferred<Unit>()
        var opened = 0
        val binding = AudioBinding(AudioPreferences(), {}, { true }, { _, _, _ ->
            object : LifecycleRun {
                override suspend fun execute() { opened++; finish.await() }
                override fun cancel() {}
                override suspend fun retire() { error("native cleanup failed") }
            }
        }, commands, scope, {}, 1, {})
        try {
            binding.start()
            val endpoint = AudioEndpoint("a".repeat(64), 1234, 1, 1, 20, false)
            commands.value = endpoint
            finish.complete(Unit)
            assertTrue(errors.isEmpty())
            assertFalse(binding.sharing)
            assertTrue(binding.status.contains("native cleanup failed"))
            commands.value = endpoint.copy(port = 1235)
            assertEquals("T807 uncertain native owner was replaced", 1, opened)
            assertFalse(binding.sharing)
            assertTrue(binding.status.contains("cleanup incomplete"))
        } finally { binding.shutdown(); scope.cancel() }
    }

}
