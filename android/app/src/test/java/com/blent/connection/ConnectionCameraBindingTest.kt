package com.blent.connection

import com.blent.ResourceRetirement
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionCameraBindingTest {
    private val spec = ConnectionWorkerSpec("test")
    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val done = CompletableDeferred<Unit>()
        val released = CompletableDeferred<Unit>()
        val commands = CopyOnWriteArrayList<ConnectionWorkerCommand>()
        override suspend fun execute() = done.await()
        override fun cancel() {}
        override suspend fun retire() = released.await()
        override fun command(value: ConnectionWorkerCommand): Boolean = commands.add(value)
    }
    private inner class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        val binding = ConnectionBinding(scope, { _, report -> Worker(report).also { workers.add(it) } })
        fun connect(): Worker {
            binding.connect(spec)
            return workers.last().also { authenticate(it) }
        }
        fun authenticate(worker: Worker) {
            worker.report(ConnectionWorkerEvent.Peer(cameraLease().connection, ConnectionPeer.parse("cd".repeat(32)), false))
            worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        }
        fun admit(worker: Worker) {
            worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
        }
        override fun close() {
            workers.forEach { it.released.complete(Unit) }
            binding.disconnect(); scope.cancel()
        }
    }
    private suspend fun until(done: () -> Boolean) = withTimeout(2000) { while (!done()) delay(1) }

    @Test fun t948ReplacementRevokesTargetButLateClaimsRelayOnlyToOriginalWorker() = runBlocking {
        Rig().use { rig ->
            val original = rig.connect(); rig.admit(original)
            val target = checkNotNull(rig.binding.cameraTarget.value)
            val first = checkNotNull(target.authority.retain())
            val second = checkNotNull(target.authority.retain())
            rig.binding.connect(spec)
            assertNull(rig.binding.cameraTarget.value); assertFalse(first.current()); assertEquals(1, rig.workers.size)
            original.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
            first.retired(ResourceRetirement.NOT_STARTED); assertTrue(original.commands.isEmpty())
            second.retired(ResourceRetirement.RETIRED)
            until { original.commands.isNotEmpty() }
            assertEquals(listOf(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())), original.commands)
            original.released.complete(Unit); until { rig.workers.size == 2 }
            val next = rig.workers.last(); rig.authenticate(next); rig.admit(next)
            val nextTarget = checkNotNull(rig.binding.cameraTarget.value)
            assertNotSame(target, nextTarget)
            val nextClaim = checkNotNull(nextTarget.authority.retain())
            original.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
            original.report(ConnectionWorkerEvent.Ended("old failure"))
            first.retired(ResourceRetirement.RETIRED)
            assertSame(nextTarget, rig.binding.cameraTarget.value); assertTrue(nextClaim.current()); assertTrue(next.commands.isEmpty())
            rig.binding.disconnect(); assertNull(rig.binding.cameraTarget.value); assertFalse(nextClaim.current())
            next.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
            nextClaim.retired(ResourceRetirement.RETIRED)
            until { next.commands.size == 1 }
        }
    }

    @Test fun t948ConnectionExecutionEndIsNotCaptureProofAndStaleRoutesCannotReopen() = runBlocking {
        Rig().use { rig ->
            val worker = rig.connect(); rig.admit(worker)
            val target = checkNotNull(rig.binding.cameraTarget.value)
            val claim = checkNotNull(target.authority.retain())
            worker.done.complete(Unit)
            assertNull(rig.binding.cameraTarget.value); assertFalse(claim.current()); assertFalse(worker.released.isCompleted)
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute(2))); assertNull(rig.binding.cameraTarget.value)
            worker.report(ConnectionWorkerEvent.CameraClosed(cameraLease())); assertTrue(worker.commands.isEmpty())
            claim.retired(ResourceRetirement.RETIRED)
            until { worker.commands.size == 1 }
            worker.released.complete(Unit)
            until { rig.binding.state.value.cleanup == null }
            assertFalse(target.authority.stop())
        }
    }

    @Test fun t948ForeignAndUnauthenticatedRoutesNeverCreateNativeTargetAndCameraClosePreservesDisplay() = runBlocking {
        Rig().use { rig ->
            rig.binding.connect(spec); val worker = rig.workers.single()
            rig.admit(worker); assertNull(rig.binding.cameraTarget.value)
            rig.authenticate(worker)
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute().copy(lease = cameraLease().copy(connection =
                ConnectionTicket(2, cameraLease().connection.peer)))))
            assertNull(rig.binding.cameraTarget.value)
            worker.report(ConnectionWorkerEvent.Display(displayRoute()))
            val display = rig.binding.state.value.display
            rig.admit(worker)
            val target = checkNotNull(rig.binding.cameraTarget.value)
            worker.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
            until { worker.commands.size == 1 }
            assertNull(target.authority.retain()); assertNull(rig.binding.cameraTarget.value)
            assertSame(display, rig.binding.state.value.display); assertTrue(rig.binding.state.value.authenticated)
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute())); assertNull(rig.binding.cameraTarget.value)
        }
    }

    @Test fun t948PermissionAndChoiceRemainBoundToOriginalPublicationAndPrompt() {
        Rig().use { rig ->
            val forged = ConnectionCameraOfferChoice(LocalCameraOffer(cameraLease(), cameraProfile(), true))
            val context = CameraConsentChoice(true, true)
            assertFalse(rig.binding.cameraChoice(forged, context)); assertFalse(rig.binding.cameraPermission(forged, true, context))
            val worker = rig.connect()
            worker.report(ConnectionWorkerEvent.CameraOffered(forged.offer))
            val offer = checkNotNull(rig.binding.state.value.cameraOffer)
            assertFalse(rig.binding.cameraChoice(forged, context)); assertFalse(rig.binding.cameraPermission(forged, true, context))
            assertTrue(rig.binding.cameraChoice(offer, context)); assertTrue(rig.binding.cameraPermission(offer, false, context))
            assertEquals(listOf(ConnectionWorkerCommand.CameraChoice(cameraLease(), context),
                ConnectionWorkerCommand.CameraPermission(cameraLease(), false, context)), worker.commands)
            rig.binding.disconnect()
            assertFalse(rig.binding.cameraChoice(offer, context)); assertFalse(rig.binding.cameraPermission(offer, true, context))
        }
    }
}
