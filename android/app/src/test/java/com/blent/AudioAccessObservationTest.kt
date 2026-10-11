package com.blent

import com.blent.connection.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class AudioAccessObservationTest {
    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val release = CompletableDeferred<Unit>()
        val commands = mutableListOf<ConnectionWorkerCommand>()
        override suspend fun execute() = awaitCancellation()
        override fun cancel() {}
        override suspend fun retire() = release.await()
        override fun command(value: ConnectionWorkerCommand) = commands.add(value)
    }
    private class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        val connection = ConnectionBinding(scope, { _, report -> Worker(report).also { workers.add(it) } })
        val nativeRelease = CompletableDeferred<Unit>()
        var opened = 0; var cancelled = 0
        val binding = AudioBinding(AudioPreferences(), {}, { true }, { _, _, _ -> object : LifecycleRun {
            override suspend fun execute() { opened++; awaitCancellation() }
            override fun cancel() { if (cancelled == 0) cancelled++ } // Native cancellation is idempotent.
            override suspend fun retire() = nativeRelease.await()
        } }, MutableStateFlow(null), scope, {}, 2, {})
        val observation = AudioAccessObservation(binding, ConnectionAudioAccess(connection, ConnectionManagement.Resource.SPEAKERS), LocalPermissionPrompts(), scope)
        fun connect(): Worker {
            connection.connect(ConnectionWorkerSpec("test"))
            return workers.last().also {
                it.report(ConnectionWorkerEvent.Peer(audioLease().connection, ConnectionPeer.parse("cd".repeat(32)), false))
                it.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            }
        }
        override fun close() {
            nativeRelease.complete(Unit); workers.forEach { it.release.complete(Unit) }
            observation.retire(); binding.shutdown(); connection.disconnect(); scope.cancel()
        }
    }
    @Test fun t985LocalStartAdmissionWithdrawalAndActualAudioCleanupStayWithOriginalWorker() = runBlocking {
        Rig().use { rig ->
            rig.binding.start(); rig.observation.policy.foreground(true)
            val worker = rig.connect(); val route = audioRoute()
            worker.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)))
            val offer = checkNotNull(rig.observation.policy.state.value.offer)
            assertEquals(0, rig.opened); assertFalse(rig.binding.sharing)
            assertTrue(rig.observation.policy.start(offer))
            worker.report(ConnectionWorkerEvent.AudioRoute(route))
            assertEquals(1, rig.opened); assertTrue(rig.binding.sharing)
            assertSame(offer.owner, rig.observation.policy.state.value.offer!!.owner)
            worker.report(ConnectionWorkerEvent.AudioClosed(route.lease))
            assertNull(rig.observation.policy.state.value.offer); assertFalse(rig.binding.sharing)
            assertEquals(1, rig.cancelled)
            assertFalse(worker.commands.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            rig.nativeRelease.complete(Unit)
            withTimeout(2000) { while (worker.commands.none { it is ConnectionWorkerCommand.AudioConsumerRetired }) delay(1) }
            assertEquals(ConnectionWorkerCommand.AudioConsumerRetired(route.lease), worker.commands.last())
        }
    }
    @Test fun t985SourceLossAndObserverRetirementNeverGrantReplacementOrConsumeOriginalCleanup() = runBlocking {
        Rig().use { rig ->
            rig.binding.start(); rig.observation.policy.foreground(true)
            val worker = rig.connect(); val route = audioRoute()
            worker.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)))
            val offer = checkNotNull(rig.observation.policy.state.value.offer)
            assertTrue(rig.observation.policy.start(offer)); worker.report(ConnectionWorkerEvent.AudioRoute(route))
            rig.connection.disconnect()
            assertNull(rig.observation.policy.state.value.offer); assertFalse(rig.binding.sharing)
            assertFalse(rig.observation.policy.start(offer)); assertEquals(1, rig.opened)
            assertFalse(worker.commands.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            worker.report(ConnectionWorkerEvent.AudioClosed(route.lease)); rig.nativeRelease.complete(Unit)
            withTimeout(2000) { while (worker.commands.none { it is ConnectionWorkerCommand.AudioConsumerRetired }) delay(1) }
            rig.observation.retire(); rig.observation.retire()
            assertFalse(rig.observation.policy.state.value.enabled)
        }
    }
}
