package com.blent.connection

import com.blent.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.collect
import org.junit.Assert.*
import org.junit.Test

class ConnectionCameraAccessTest {
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
        val binding = ConnectionBinding(scope, { _, report -> Worker(report).also { workers.add(it) } })
        val access = ConnectionCameraAccess(binding)
        fun connect(): Worker {
            binding.connect(ConnectionWorkerSpec("test"))
            return workers.last().also {
                it.report(ConnectionWorkerEvent.Peer(cameraLease().connection, ConnectionPeer.parse("cd".repeat(32)), false))
                it.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            }
        }
        override fun close() { workers.forEach { it.release.complete(Unit) }; binding.disconnect(); scope.cancel() }
    }
    private val consent = CameraLocalConsent(true, false, true, false)

    @Test fun t932CaptureProjectionJoinsExactOriginalTargetAndDropsMismatchedMetadata() = Rig().use { rig ->
        val emitted = mutableListOf<CameraCapturePublication?>()
        val observer = rig.scope.launch { rig.access.captures.collect { emitted.add(it) } }
        try {
            assertNull(rig.access.capture())
            val worker = rig.connect()
            worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
            assertNull(rig.access.capture())
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
            val original = rig.access.capture()!!
            assertSame(rig.access.project()!!.owner, original.owner)
            assertSame(rig.binding.cameraTarget.value, original.target)
            worker.report(ConnectionWorkerEvent.Trust(cameraLease().connection.peer, true))
            assertSame(original.owner, rig.access.capture()!!.owner)
            assertSame(original.target, rig.access.capture()!!.target)
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute(2).copy(port = 12346)))
            assertNull(rig.access.capture())
            assertNull(emitted.last())
            worker.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
            assertNull(rig.access.capture())
        } finally { observer.cancel() }
    }

    @Test fun t944OfferPermissionAdmissionKeepOneOwnerButExactPublicationAuthority() = Rig().use { rig ->
        assertNull(rig.access.project())
        val worker = rig.connect()
        worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
        val first = rig.access.project()!!
        assertEquals(CameraLens.REAR, first.lens)
        assertFalse(first.permissionRequired); assertFalse(first.admitted); assertFalse(first.backgroundRequested)
        assertSame(first, rig.access.project())
        assertTrue(first.authority.update(consent))
        assertFalse(first.authority.permissionResult(true, consent))
        worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), true)))
        val permission = rig.access.project()!!
        assertSame(first.owner, permission.owner); assertTrue(permission.permissionRequired)
        assertFalse(first.authority.current()); assertFalse(first.authority.update(consent))
        assertTrue(permission.authority.permissionResult(false, consent))
        worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
        val admitted = rig.access.project()!!
        assertSame(first.owner, admitted.owner); assertTrue(admitted.admitted)
        assertFalse(permission.authority.permissionResult(true, consent))
        assertFalse(admitted.authority.permissionResult(true, consent))
        assertTrue(admitted.authority.update(consent.copy(permission = true)))
    }

    @Test fun t944IdenticalTicketsInReplacementPeerCannotReviveOriginalAccess() = Rig().use { rig ->
        val firstWorker = rig.connect()
        firstWorker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), true)))
        val original = rig.access.project()!!
        rig.binding.disconnect(); assertFalse(original.authority.current())
        assertFalse(original.authority.update(consent)); assertNull(rig.access.project())
        firstWorker.release.complete(Unit)
        val next = rig.connect()
        next.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), true)))
        val replacement = rig.access.project()!!
        assertNotSame(original.owner, replacement.owner)
        assertFalse(original.authority.permissionResult(true, consent)); assertTrue(replacement.authority.current())
        next.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(2), cameraProfile(), false)))
        assertNotSame(replacement.owner, rig.access.project()!!.owner)
    }

    @Test fun t944InvalidVisibilityForeignLeaseAndSourceLossCannotExposeAccess() = Rig().use { rig ->
        val worker = rig.connect()
        worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
        val view = rig.binding.state.value
        for (bad in listOf(view.copy(active = false), view.copy(authenticated = false), view.copy(peer = null),
            view.copy(peer = ConnectionChoice(ConnectionTicket(2, cameraLease().connection.peer), view.peer!!.tablet)))) {
            assertNull(rig.access.project(bad))
        }
        val original = rig.access.project()!!
        worker.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
        assertNull(rig.access.project()); assertFalse(original.authority.current())
    }

    @Test fun t944EveryConsentBooleanMapsWithoutGrantingCaptureOrChangingResource() = Rig().use { rig ->
        val worker = rig.connect()
        worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile().copy(lens = 0, background = true), true)))
        val access = rig.access.project()!!
        assertEquals(CameraLens.FRONT, access.lens); assertTrue(access.backgroundRequested)
        for (bits in 0 until 16) {
            val context = CameraLocalConsent(bits and 1 != 0, bits and 2 != 0, bits and 4 != 0, bits and 8 != 0)
            assertTrue(access.authority.update(context)); assertTrue(access.authority.permissionResult(bits and 1 != 0, context))
            assertEquals(ConnectionWorkerCommand.CameraChoice(cameraLease(), CameraConsentChoice(context.foreground,
                context.permission, context.enabled, context.backgroundAllowed)), worker.commands[bits * 2])
            assertEquals(ConnectionWorkerCommand.CameraPermission(cameraLease(), context.foreground, CameraConsentChoice(context.foreground,
                context.permission, context.enabled, context.backgroundAllowed)), worker.commands[bits * 2 + 1])
        }
        assertNull(rig.binding.cameraTarget.value)
    }
}
