package com.blent.connection

import com.blent.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.collect
import org.junit.Assert.*
import org.junit.Test

class ConnectionAudioAccessTest {
    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val release = CompletableDeferred<Unit>()
        val commands = mutableListOf<ConnectionWorkerCommand>()
        override suspend fun execute() = awaitCancellation()
        override fun cancel() {}
        override suspend fun retire() = release.await()
        override fun command(value: ConnectionWorkerCommand) = commands.add(value)
    }
    private class Rig(val resource: ConnectionManagement.Resource) : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        val binding = ConnectionBinding(scope, { _, report -> Worker(report).also { workers.add(it) } })
        val access = ConnectionAudioAccess(binding, resource)
        fun connect() : Worker {
            binding.connect(ConnectionWorkerSpec("test"))
            return workers.last().also {
                it.report(ConnectionWorkerEvent.Peer(audioLease(resource).connection, ConnectionPeer.parse("cd".repeat(32)), false))
                it.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            }
        }
        override fun close() { workers.forEach { it.release.complete(Unit) }; binding.disconnect(); scope.cancel() }
    }
    private val resources = listOf(ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS)
    private val context = MediaLocalConsent(true, true, true, false)
    @Test fun t985BothDirectionsKeepOriginalOfferIdentityButRequireExactPublication() {
        for (resource in resources) Rig(resource).use { rig ->
            val observed = mutableListOf<AudioAccessOffer?>()
            val observer = rig.scope.launch { rig.access.offers.collect { observed.add(it) } }
            try {
                assertNull(rig.access.project()); assertNull(rig.access.capture())
                val worker = rig.connect(); val route = audioRoute(resource)
                worker.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)))
                val original = checkNotNull(rig.access.project())
                assertSame(original, rig.access.project()); assertFalse(original.admitted)
                assertEquals(if (resource == resources.first()) 1 else 2, original.direction)
                assertFalse(original.permissionRequired); assertFalse(original.authority.permissionResult(true, context))
                assertTrue(original.authority.update(context)); assertNull(rig.access.capture())
                worker.report(ConnectionWorkerEvent.AudioRoute(route))
                val admitted = checkNotNull(rig.access.project()); assertSame(original.owner, admitted.owner)
                assertTrue(admitted.admitted); assertFalse(original.authority.current())
                assertFalse(original.authority.update(context)); assertTrue(admitted.authority.update(context))
                assertSame(admitted, observed.last()); assertSame(admitted.owner, rig.access.capture()!!.owner)
                assertFalse(admitted.authority.permissionResult(false, context))
            } finally { observer.cancel() }
        }
    }
    @Test fun t985CaptureWithdrawalStaleMetadataAndSiblingEventsRemainIndependent() {
        for (resource in resources) Rig(resource).use { rig ->
            val observed = mutableListOf<MediaConsumerPublication<AudioConsumerTarget>?>()
            val observer = rig.scope.launch { rig.access.captures.collect { observed.add(it) } }
            try {
                val worker = rig.connect(); val route = audioRoute(resource)
                worker.report(ConnectionWorkerEvent.AudioRoute(route))
                val capture = checkNotNull(rig.access.capture())
                val sibling = if (resource == resources.first()) resources.last() else resources.first()
                worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute(sibling)))
                worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
                assertSame(capture.target, rig.access.capture()!!.target)
                assertSame(capture.owner, rig.access.capture()!!.owner)
                val replacement = route.copy(lease = audioLease(resource, 2), port = route.port + 1)
                worker.report(ConnectionWorkerEvent.AudioRoute(replacement))
                assertNull(rig.access.capture()); assertNull(observed.last())
                worker.report(ConnectionWorkerEvent.AudioClosed(route.lease))
                worker.report(ConnectionWorkerEvent.AudioClosed(replacement.lease))
                assertNull(rig.access.project()); assertNull(rig.access.capture())
            } finally { observer.cancel() }
        }
    }
    @Test fun t985ReplacementPeerNeverInheritsLocalConsentEvenWithIdenticalLeaseValues() {
        for (resource in resources) Rig(resource).use { rig ->
            val worker = rig.connect(); val route = audioRoute(resource)
            worker.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)))
            val original = checkNotNull(rig.access.project())
            rig.binding.disconnect(); assertFalse(original.authority.current()); assertNull(rig.access.project())
            worker.release.complete(Unit)
            val replacement = rig.connect()
            replacement.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile.copy(background = true))))
            val next = checkNotNull(rig.access.project()); assertNotSame(original.owner, next.owner)
            assertTrue(next.backgroundRequested); assertFalse(original.authority.update(context))
            replacement.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(audioLease(resource, 2), route.profile)))
            assertNotSame(next.owner, rig.access.project()!!.owner)
        }
    }
    @Test fun t985InvalidVisibilityResourceAndEveryConsentFlagStayInsideExactDirection() {
        for (resource in resources) Rig(resource).use { rig -> checkInvalid(rig) }
        for (resource in listOf(ConnectionManagement.Resource.DISPLAY, ConnectionManagement.Resource.CAMERA)) {
            Rig(resources.last()).use { rig -> assertThrows(IllegalArgumentException::class.java) { ConnectionAudioAccess(rig.binding, resource) } }
        }
    }
    private fun checkInvalid(rig: Rig) {
        val worker = rig.connect(); val route = audioRoute(rig.resource)
        worker.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)))
        val view = rig.binding.state.value
        val foreign = ConnectionChoice(view.peer!!.ticket.copy(epoch = 2), view.peer.tablet)
        for (bad in listOf(view.copy(active = false), view.copy(authenticated = false), view.copy(peer = null), view.copy(peer = foreign))) {
            assertNull(rig.access.project(bad))
        }
        val offer = checkNotNull(rig.access.project())
        for (bits in 0..15) {
            val value = MediaLocalConsent(bits and 1 != 0, bits and 2 != 0, bits and 4 != 0, bits and 8 != 0)
            assertTrue(offer.authority.update(value))
            assertEquals(ConnectionWorkerCommand.AudioChoice(route.lease, value), worker.commands.last())
        }
    }
}
