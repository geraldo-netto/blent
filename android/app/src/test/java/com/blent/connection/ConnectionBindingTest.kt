package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionBindingTest {
    private val spec = ConnectionWorkerSpec("selected by admission")
    private val peer = ConnectionPeer.parse("ab".repeat(32))
    private val local = ConnectionPeer.parse("cd".repeat(32))
    private val ticket = ConnectionTicket(1, peer)

    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val done = CompletableDeferred<Unit>()
        val released = CompletableDeferred<Unit>()
        val commands = mutableListOf<ConnectionWorkerCommand>()
        var cancelled = 0
        var failure: Exception? = null
        var rejects = false
        var controlFailure: Exception? = null
        override suspend fun execute() { done.await() }
        override fun cancel() { cancelled++ }
        override suspend fun retire() { released.await(); failure?.let { throw it } }
        override fun command(value: ConnectionWorkerCommand): Boolean {
            controlFailure?.let { throw it }; commands.add(value); return !rejects
        }
    }

    private inner class Rig(val deadline: Long = 5000) : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        var factoryFailure: Exception? = null
        val binding = ConnectionBinding(scope, { chosen, report ->
            assertEquals(spec, chosen)
            factoryFailure?.let { throw it }
            Worker(report).also { workers.add(it) }
        }, deadline)
        fun connect(): Worker { binding.connect(spec); return workers.last() }
        fun prompt(worker: Worker): ConnectionChoice {
            worker.report(ConnectionWorkerEvent.Pairing(ticket, local))
            return checkNotNull(binding.state.value.approval)
        }
        override fun close() {
            workers.forEach { it.released.complete(Unit) }
            binding.disconnect(); scope.cancel()
        }
    }

    @Test fun t892DefaultControllerAndInvalidCleanupNoticeLimits() {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        try {
            val binding = ConnectionBinding(scope, { _, _ -> error("must remain lazy") })
            assertFalse(binding.state.value.active)
            binding.disconnect()
            for (value in listOf(Long.MIN_VALUE, -1, 0, 60001, Long.MAX_VALUE)) {
                assertThrows(IllegalArgumentException::class.java) {
                    ConnectionBinding(scope, { _, _ -> error("must remain lazy") }, value)
                }
            }
        } finally { scope.cancel() }
    }

    @Test fun t892DecisionsAreBoundToOneDisplayedWorkerAndSavedTrustIsNotOptimistic() {
        Rig().use { rig ->
            val forged = ConnectionChoice(ticket, local)
            assertFalse(rig.binding.decide(forged, true, true)); assertFalse(rig.binding.revoke(forged))
            val worker = rig.connect(); val choice = rig.prompt(worker)
            assertFalse(rig.binding.decide(forged, true, true)); assertFalse(rig.binding.revoke(forged))
            assertTrue(rig.binding.decide(choice, true, true))
            assertEquals(listOf(ConnectionWorkerCommand.Decide(ticket, true, true)), worker.commands)
            assertFalse(rig.binding.decide(choice, true, true))
            assertFalse(checkNotNull(rig.binding.state.value.remembered))
            worker.report(ConnectionWorkerEvent.Trust(peer, true))
            assertTrue(checkNotNull(rig.binding.state.value.remembered))
            assertTrue(rig.binding.revoke(choice)); assertFalse(rig.binding.revoke(choice))
            assertEquals(ConnectionWorkerCommand.Revoke(peer), worker.commands.last())
            assertTrue(checkNotNull(rig.binding.state.value.remembered))
            worker.report(ConnectionWorkerEvent.Trust(peer, false))
            assertFalse(checkNotNull(rig.binding.state.value.remembered))
        }
    }

    @Test fun t892ReplacementWaitsForActualRetirementAndDiscardsOldChoice() = runBlocking {
        Rig(10).use { rig ->
            val first = rig.connect(); val oldChoice = rig.prompt(first)
            repeat(100) { rig.binding.connect(spec) }
            assertEquals(1, rig.workers.size); assertEquals(1, first.cancelled)
            withTimeout(2000) { while (rig.binding.state.value.cleanup?.contains("stalled") != true) delay(1) }
            first.report(ConnectionWorkerEvent.Pairing(ticket, local))
            assertNull(rig.binding.state.value.approval)
            first.released.complete(Unit)
            assertEquals(2, rig.workers.size)
            val current = rig.workers.last(); val choice = rig.prompt(current)
            assertNull(rig.binding.state.value.cleanup)
            assertFalse(rig.binding.decide(oldChoice, true, true))
            assertFalse(rig.binding.revoke(oldChoice))
            first.report(ConnectionWorkerEvent.Trust(peer, true))
            assertFalse(checkNotNull(rig.binding.state.value.remembered))
            assertTrue(rig.binding.decide(choice, false, true))
            assertEquals(ConnectionWorkerCommand.Decide(ticket, false, false), current.commands.single())
        }
    }

    @Test fun t892CleanupFailureQuarantinesWorkerInsteadOfReplacingIt() = runBlocking {
        Rig().use { rig ->
            val worker = rig.connect()
            worker.failure = IllegalStateException("release failed")
            rig.binding.connect(spec)
            worker.released.complete(Unit)
            assertTrue(rig.binding.state.value.cleanup.orEmpty().contains("release failed"))
            assertFalse("T892 quarantined replacement cannot remain shown as active", rig.binding.state.value.active)
            rig.binding.connect(spec)
            assertEquals(1, rig.workers.size)
            assertTrue(rig.binding.state.value.cleanup.orEmpty().contains("release failed"))
        }
    }

    @Test fun t892RejectedOrThrowingControlStopsWithoutClaimingRememberOrRevoke() {
        for (throws in listOf(false, true)) Rig().use { rig ->
            val worker = rig.connect(); val choice = rig.prompt(worker)
            if (throws) worker.controlFailure = IllegalStateException("control failed") else worker.rejects = true
            assertFalse(rig.binding.decide(choice, true, true))
            assertFalse(rig.binding.state.value.active)
            assertFalse(checkNotNull(rig.binding.state.value.remembered))
            assertEquals(1, worker.cancelled)
        }
    }

    @Test fun t892FactoryFailureAndCleanCompletionRetainHonestCleanupState() {
        Rig().use { rig ->
            rig.factoryFailure = IllegalStateException("cannot construct")
            rig.binding.connect(spec)
            assertEquals("cannot construct", rig.binding.state.value.status)
            assertNull(rig.binding.state.value.cleanup)
            rig.factoryFailure = null
            val worker = rig.connect()
            worker.done.complete(Unit)
            assertFalse(rig.binding.state.value.active)
            assertNotNull(rig.binding.state.value.cleanup)
            worker.released.complete(Unit)
            assertNull(rig.binding.state.value.cleanup)
            rig.binding.disconnect(); rig.binding.disconnect()
            assertNull(rig.binding.state.value.cleanup)
        }
    }
}
