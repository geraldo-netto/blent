package com.blent.connection

import com.blent.*
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

/** T1239: metadata workers and explicit retirement gates, never device evidence. */
class ConnectionReconnectStopTest {
    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val released = CompletableDeferred<Unit>()
        val commands = CopyOnWriteArrayList<ConnectionWorkerCommand>()
        override suspend fun execute() = awaitCancellation()
        override fun cancel() {}
        override suspend fun retire() = released.await()
        override fun command(value: ConnectionWorkerCommand) = commands.add(value)
    }

    private class Rig {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = CopyOnWriteArrayList<Worker>()
        val binding = ConnectionBinding(scope, { _, report -> Worker(report).also(workers::add) })
        private val camera = ConnectionCameraAccess(binding)
        private val microphone = ConnectionAudioAccess(binding, ConnectionManagement.Resource.MICROPHONE)
        private val speakers = ConnectionAudioAccess(binding, ConnectionManagement.Resource.SPEAKERS)
        val policies = List(3) { MediaConsentPolicy<MediaAccessOffer>({ true }, LocalPermissionPrompts()) }
        private val observers = listOf(
            MediaConsentObservation<MediaAccessOffer>(camera.offers, policies[0], scope),
            MediaConsentObservation<MediaAccessOffer>(microphone.offers, policies[1], scope),
            MediaConsentObservation<MediaAccessOffer>(speakers.offers, policies[2], scope),
        )

        init { policies.forEach { it.foreground(true) } }

        fun connect(): Worker {
            binding.connect(ConnectionWorkerSpec("T1239 metadata fixture"))
            return workers.last().also(::publish)
        }

        fun publish(worker: Worker) {
            worker.report(ConnectionWorkerEvent.Peer(cameraLease().connection, ConnectionPeer.parse("cd".repeat(32)), true))
            worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
            worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute(ConnectionManagement.Resource.MICROPHONE)))
            worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute(ConnectionManagement.Resource.SPEAKERS)))
        }

        fun targets(): List<MediaConsumerTarget<*>> = listOf(
            checkNotNull(binding.cameraTarget.value), checkNotNull(binding.microphoneTarget.value),
            checkNotNull(binding.speakersTarget.value),
        )

        fun closeResource(worker: Worker, index: Int) {
            worker.report(closed()[index])
        }

        suspend fun close() {
            observers.forEach { it.retire() }
            binding.disconnect()
            workers.forEach { it.released.complete(Unit) }
            scope.cancel()
            scope.coroutineContext[Job]!!.join()
        }
    }

    private companion object {
        val orders = listOf(listOf(0, 1, 2), listOf(0, 2, 1), listOf(1, 0, 2),
            listOf(1, 2, 0), listOf(2, 0, 1), listOf(2, 1, 0))

        fun closed() = listOf(ConnectionWorkerEvent.CameraClosed(cameraLease()),
            ConnectionWorkerEvent.AudioClosed(audioLease(ConnectionManagement.Resource.MICROPHONE)),
            ConnectionWorkerEvent.AudioClosed(audioLease(ConnectionManagement.Resource.SPEAKERS)))

        suspend fun settled(done: () -> Boolean) {
            withTimeout(2000) { while (!done()) delay(1) }
        }
    }

    @Test fun t1239AllStopAndReceiptOrdersKeepOriginalRetirementAndSiblingAuthority() = runBlocking {
        for (stopOrder in orders) for (receiptOrder in orders) verifyStopOrder(stopOrder, receiptOrder)
    }

    private suspend fun verifyStopOrder(stopOrder: List<Int>, receiptOrder: List<Int>) {
        val rig = Rig()
        try {
            val worker = rig.connect()
            val targets = rig.targets()
            val claims = targets.map { checkNotNull(it.authority.retain()) }
            for (index in stopOrder) {
                assertTrue(targets[index].authority.stop())
                rig.closeResource(worker, index)
                assertFalse(claims[index].current())
                assertNull(targets[index].authority.retain())
                assertEquals(stopOrder.indexOf(index) + 1, worker.commands.size)
                for (sibling in stopOrder.drop(stopOrder.indexOf(index) + 1)) assertTrue(claims[sibling].current())
            }
            for ((count, index) in receiptOrder.withIndex()) {
                claims[index].retired(ResourceRetirement.RETIRED)
                settled { worker.commands.size == 4 + count }
                claims[index].retired(ResourceRetirement.NOT_STARTED)
            }
            assertEquals(6, worker.commands.size)
            assertEquals(receiptOrder.map(::retiredCommand), worker.commands.drop(3))
            assertTrue(rig.binding.state.value.authenticated)
        } finally { rig.close() }
    }

    @Test fun t1239EqualLeasesAfterReconnectNeedFreshConsentAndRejectEveryOldStop() = runBlocking {
        for (order in orders) verifyReconnect(order)
    }

    private suspend fun verifyReconnect(receiptOrder: List<Int>) {
        val rig = Rig()
        try {
            val original = rig.connect()
            val oldOffers = rig.policies.map { checkNotNull(it.state.value.offer) }
            rig.policies.forEachIndexed { index, policy -> assertTrue(policy.start(oldOffers[index])) }
            val oldClaims = rig.targets().map { checkNotNull(it.authority.retain()) }
            rig.binding.connect(ConnectionWorkerSpec("T1239 replacement"))
            assertEquals(1, rig.workers.size)
            assertTrue(oldClaims.none { it.current() })
            original.released.complete(Unit)
            settled { rig.workers.size == 2 }
            val replacement = rig.workers.last()
            rig.publish(replacement)
            val nextOffers = rig.policies.map { checkNotNull(it.state.value.offer) }
            for (index in receiptOrder) {
                verifyFreshConsent(rig.policies[index], oldOffers[index], nextOffers[index])
                original.report(closed()[index])
                oldClaims[index].retired(ResourceRetirement.RETIRED)
            }
            assertTrue(replacement.commands.isEmpty())
            rig.policies.forEachIndexed { index, policy -> assertTrue(policy.start(nextOffers[index])) }
            assertEquals(3, replacement.commands.size)
            rig.policies.forEachIndexed { index, policy -> assertTrue(policy.stop(nextOffers[index])) }
            assertEquals(6, replacement.commands.size)
            assertTrue(rig.binding.state.value.authenticated)
        } finally { rig.close() }
    }

    private fun verifyFreshConsent(policy: MediaConsentPolicy<MediaAccessOffer>, old: MediaAccessOffer, next: MediaAccessOffer) {
        assertNotSame(old.owner, next.owner)
        assertFalse(policy.state.value.enabled)
        assertFalse(policy.start(old))
        assertFalse(policy.stop(old))
        assertFalse(old.authority.update(MediaLocalConsent(true, true, false, false)))
        assertTrue(next.authority.current())
    }

    private fun retiredCommand(index: Int): ConnectionWorkerCommand = when (index) {
        0 -> ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())
        1 -> ConnectionWorkerCommand.AudioConsumerRetired(audioLease(ConnectionManagement.Resource.MICROPHONE))
        else -> ConnectionWorkerCommand.AudioConsumerRetired(audioLease(ConnectionManagement.Resource.SPEAKERS))
    }
}
