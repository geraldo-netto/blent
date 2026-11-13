package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConfiguredAttachmentTest {
    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val ended = CompletableDeferred<Unit>()
        val retired = CompletableDeferred<Unit>()
        val commands = mutableListOf<ConnectionWorkerCommand>()
        override suspend fun execute() { ended.await() }
        override fun cancel() { ended.complete(Unit) }
        override suspend fun retire() { retired.await() }
        override fun command(value: ConnectionWorkerCommand): Boolean { commands.add(value); return true }
    }
    private class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        val binding = ConnectionBinding(scope, { _, report -> Worker(report).also(workers::add) })
        val preferences = ConnectionPreferenceStore(PreferenceMemoryStorage(), Dispatchers.Unconfined)
        val configured = ConfiguredConnection(binding, preferences, scope)
        fun connect(): Worker { configured.connect(); return workers.last() }
        override fun close() {
            configured.disconnect()
            workers.forEach { it.ended.complete(Unit); it.retired.complete(Unit) }
            scope.cancel()
            runBlocking { scope.coroutineContext[Job]!!.join() }
        }
    }

    @Test fun t1392CoalescedAttachmentWaitsForOriginalNativeRetirement() {
        Rig().use { rig ->
            val original = rig.connect()
            repeat(8) { rig.configured.attach() }
            assertEquals(1, rig.workers.size)
            original.ended.complete(Unit)
            assertEquals("T1392: logical completion is not retirement", 1, rig.workers.size)
            original.retired.complete(Unit)
            assertEquals(2, rig.workers.size)
            assertTrue(rig.workers.last().commands.isEmpty())
        }
    }

    @Test fun t1392ExplicitDisconnectCancelsPendingAttachment() {
        Rig().use { rig ->
            val original = rig.connect()
            rig.configured.attach()
            rig.configured.disconnect()
            original.retired.complete(Unit)
            assertEquals(1, rig.workers.size)
            assertFalse(rig.configured.state.value.pending)
            rig.configured.attach()
            assertEquals("T1392: a later new attachment may start", 2, rig.workers.size)
        }
    }

    @Test fun t1392DenialAndRevocationPreventPendingAutomaticRestart() {
        for (revoke in listOf(false, true)) {
            Rig().use { rig ->
                val original = rig.connect()
                val ticket = ConnectionTicket(1, ConnectionPeer.parse("ab".repeat(32)))
                original.report(ConnectionWorkerEvent.Pairing(ticket, ConnectionPeer.parse("cd".repeat(32))))
                rig.configured.attach()
                val choice = checkNotNull(rig.binding.state.value.peer)
                if (revoke) assertTrue(rig.binding.revoke(choice))
                else assertTrue(rig.binding.decide(choice, false, false))
                original.ended.complete(Unit); original.retired.complete(Unit)
                assertEquals("T1392: explicit trust cutoff must win", 1, rig.workers.size)
                assertFalse(rig.configured.state.value.pending)
            }
        }
    }

    @Test fun t1392AttachmentScopeLossCannotStartReplacement() {
        Rig().use { rig ->
            val original = rig.connect()
            val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
            val configured = ConfiguredConnection(rig.binding, rig.preferences, scope)
            try {
                configured.attach()
                assertTrue(configured.state.value.pending)
                scope.cancel()
                original.ended.complete(Unit); original.retired.complete(Unit)
                assertEquals(1, rig.workers.size)
                assertFalse(configured.state.value.pending)
            } finally { scope.cancel() }
        }
    }

    @Test fun t1392OldAttachmentCannotRestartAfterAnInterveningAttempt() {
        Rig().use { rig ->
            val original = rig.connect()
            val dispatcher = ConnectionActorScheduler()
            val scope = CoroutineScope(SupervisorJob() + dispatcher)
            val configured = ConfiguredConnection(rig.binding, rig.preferences, scope)
            try {
                configured.attach(); dispatcher.runCurrent()
                rig.binding.disconnect()
                rig.binding.connect(ConnectionWorkerSpec("replacement"))
                original.retired.complete(Unit)
                val replacement = rig.workers.last()
                replacement.ended.complete(Unit); replacement.retired.complete(Unit)
                dispatcher.runCurrent()
                assertEquals("T1392: stale attachment crossed an explicit cutoff", 2, rig.workers.size)
            } finally { scope.cancel(); dispatcher.runCurrent() }
        }
    }


    @Test fun t1392DelayedPreferencesCannotRetargetAnAttachment() {
        Rig().use { rig ->
            val dispatcher = ConnectionActorScheduler()
            val store = ConnectionPreferenceStore(PreferenceMemoryStorage(), dispatcher)
            val configured = ConfiguredConnection(rig.binding, store, rig.scope)
            configured.attach()
            assertTrue(configured.state.value.pending)
            val intervening = rig.connect()
            intervening.ended.complete(Unit); intervening.retired.complete(Unit)
            dispatcher.runCurrent()
            assertEquals("T1392: delayed settings crossed attachment identity", 1, rig.workers.size)
            assertFalse(configured.state.value.pending)
            assertTrue(dispatcher.empty())
        }
    }


    @Test fun t1392AttachmentIdentityIsCapturedBeforeCoroutineDispatch() {
        Rig().use { rig ->
            val original = rig.connect()
            val dispatcher = ConnectionActorScheduler()
            val scope = CoroutineScope(SupervisorJob() + dispatcher)
            val configured = ConfiguredConnection(rig.binding, rig.preferences, scope)
            try {
                configured.attach()
                rig.binding.disconnect()
                rig.binding.connect(ConnectionWorkerSpec("replacement"))
                original.retired.complete(Unit)
                val replacement = rig.workers.last()
                replacement.ended.complete(Unit); replacement.retired.complete(Unit)
                dispatcher.runCurrent()
                assertEquals("T1392: event identity was captured after dispatch", 2, rig.workers.size)
            } finally { scope.cancel(); dispatcher.runCurrent() }
        }
    }

}
