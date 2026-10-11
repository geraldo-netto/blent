package com.blent.connection

import com.blent.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionAudioOwnershipTest {
    private val resources = listOf(ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS)
    @Test fun t989OriginalDirectionClaimsJoinClosureAndActualCompletionWithoutCrossingRoutes() = runBlocking {
        for (resource in resources) for (closeFirst in listOf(false, true)) {
            val sent = mutableListOf<ConnectionWorkerCommand>()
            val session = ConnectionAudioSession(resource, { sent.add(it); true }, Dispatchers.Unconfined)
            val route = audioRoute(resource); val target = checkNotNull(session.admitted(route))
            try {
                assertEquals(ConnectionAudioTarget.endpoint(route), target.endpoint)
                assertEquals(if (resource == ConnectionManagement.Resource.MICROPHONE) 1 else 2, target.endpoint.direction)
                assertTrue(target.endpoint.valid())
                val first = checkNotNull(target.authority.retain()); val second = checkNotNull(target.authority.retain())
                assertFalse(session.closed(route.lease.copy(owner = 2)))
                if (closeFirst) assertTrue(session.closed(route.lease))
                first.retired(ResourceRetirement.RETIRED); first.retired(ResourceRetirement.RETIRED)
                assertTrue(sent.isEmpty()); second.retired(ResourceRetirement.NOT_STARTED)
                if (!closeFirst) { assertTrue(sent.isEmpty()); assertTrue(session.closed(route.lease)) }
                assertEquals(listOf(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)), sent)
                assertFalse(first.current()); assertNull(target.authority.retain())
                assertFalse(session.closed(route.lease)); assertNull(session.admitted(route))
            } finally { session.retire() }
        }
    }
    @Test fun t989StopAndVisibilityLossNeverInventClosureOrReleaseActiveClaims() = runBlocking {
        for (resource in resources) {
            val sent = mutableListOf<ConnectionWorkerCommand>()
            val session = ConnectionAudioSession(resource, { sent.add(it); true }, Dispatchers.Unconfined)
            val route = audioRoute(resource); val target = checkNotNull(session.admitted(route))
            try {
                val claim = checkNotNull(target.authority.retain())
                assertTrue(target.authority.stop()); assertFalse(claim.current()); assertNull(target.authority.retain())
                assertEquals(listOf(ConnectionWorkerCommand.AudioChoice(route.lease, MediaLocalConsent(false, false, false, false))), sent)
                session.clear(); claim.retired(ResourceRetirement.RETIRED)
                assertEquals(1, sent.size)
                assertTrue(session.closed(route.lease)); assertEquals(ConnectionWorkerCommand.AudioConsumerRetired(route.lease), sent.last())
                session.stop(); assertNull(session.admitted(route.copy(lease = audioLease(resource, 2))))
            } finally { session.retire() }
        }
    }
    @Test fun t989MetadataReplayWrongDirectionTicketAndUnretiredReplacementCannotChangeOriginalOwner() = runBlocking {
        for (resource in resources) {
            val session = ConnectionAudioSession(resource, { true }, Dispatchers.Unconfined)
            val route = audioRoute(resource); val target = checkNotNull(session.admitted(route))
            try {
                assertSame(target, session.admitted(route.copy(credential = ConnectionManagement.Credential(route.credential.bytes()))))
                assertNull(session.admitted(route.copy(port = route.port + 1)))
                assertNull(session.admitted(route.copy(profile = route.profile.copy(bufferMs = 50))))
                assertNull(session.admitted(route.copy(credential = ConnectionManagement.Credential(ByteArray(32) { 99 }))))
                val other = if (resource == resources.first()) resources.last() else resources.first()
                assertNull(session.admitted(audioRoute(other)))
                assertNull(session.admitted(route.copy(lease = route.lease.copy(connection = route.lease.connection.copy(epoch = 2)))))
                val claim = checkNotNull(target.authority.retain())
                val next = route.copy(lease = audioLease(resource, 2))
                assertNull(session.admitted(next)); session.closed(route.lease); assertNull(session.admitted(next))
                claim.retired(ResourceRetirement.RETIRED); val replacement = checkNotNull(session.admitted(next))
                assertNotSame(target, replacement); assertNull(session.admitted(route))
                session.closed(next.lease)
                val maximum = route.copy(lease = audioLease(resource, Long.MAX_VALUE))
                assertNotNull(session.admitted(maximum)); session.closed(maximum.lease)
                assertNull(session.admitted(maximum)); assertNull(session.admitted(next))
            } finally { session.retire() }
        }
    }
    @Test fun t989ProofRetryStaysWithOriginalWorkerAndIndependentDirectionProgresses() = runBlocking {
        val accepted = CompletableDeferred<Unit>(); val sent = java.util.concurrent.CopyOnWriteArrayList<ConnectionWorkerCommand>()
        val attempts = java.util.concurrent.atomic.AtomicInteger()
        val microphone = ConnectionAudioSession(resources.first(), { sent.add(it); if (attempts.incrementAndGet() < 3) false else { accepted.complete(Unit); true } })
        val speakers = ConnectionAudioSession(resources.last(), { true }, Dispatchers.Unconfined)
        try {
            val mic = audioRoute(resources.first()); val speaker = audioRoute(resources.last())
            val held = checkNotNull(microphone.admitted(mic)); val claim = checkNotNull(held.authority.retain())
            val independent = checkNotNull(speakers.admitted(speaker)); assertNotNull(independent.authority.retain())
            microphone.closed(mic.lease); claim.retired(ResourceRetirement.RETIRED)
            withTimeout(2000) { accepted.await() }
            assertEquals(List(3) { ConnectionWorkerCommand.AudioConsumerRetired(mic.lease) }, sent)
            assertNotNull(independent.authority.retain())
            withTimeout(2000) { while (microphone.admitted(mic) != null) delay(1) }
        } finally { microphone.retire(); speakers.retire() }
    }
    @Test fun t989NativeWorkerRetirementTerminatesOnlyItsRetainedSessionAndLateClaimsCannotReviveIt() = runBlocking {
        for (resource in resources) {
            val session = ConnectionAudioSession(resource, { error("T989 retired worker cannot receive commands") }, Dispatchers.Unconfined)
            val route = audioRoute(resource); val target = checkNotNull(session.admitted(route))
            val claim = checkNotNull(target.authority.retain())
            session.retire(); session.retire(); claim.retired(ResourceRetirement.RETIRED)
            assertFalse(claim.current()); assertFalse(target.authority.stop()); assertFalse(session.closed(route.lease))
            assertNull(session.admitted(route)); assertNull(target.authority.retain())
        }
    }
    @Test fun t989CommandAdaptersValidateExactDirectionAndBoundsBeforeAnyRelay() = runBlocking {
        for (resource in resources) {
            val commands = ConnectionAudioConsumerCommands(resource)
            val foreign = audioLease(if (resource == resources.first()) resources.last() else resources.first())
            assertThrows(IllegalArgumentException::class.java) { commands.stop(foreign) }
            assertThrows(IllegalArgumentException::class.java) { commands.retired(foreign) }
            assertThrows(IllegalArgumentException::class.java) { ConnectionMediaCompletionRelay(commands, foreign, { true }) }
            val relay = ConnectionMediaCompletionRelay(commands, audioLease(resource), { false }, Dispatchers.Unconfined, 1)
            relay.ready(); relay.retire(); assertFalse(relay.stop())
        }
        for (resource in listOf(ConnectionManagement.Resource.DISPLAY, ConnectionManagement.Resource.CAMERA)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionAudioConsumerCommands(resource) }
            assertThrows(IllegalArgumentException::class.java) { ConnectionAudioPresentation(resource) }
        }
    }
    @Test fun t989BoundedSeededClaimOrderKeepsAllActualConsumersUntilOneOriginalProof() = runBlocking {
        val random = java.util.Random(989)
        repeat(128) {
            val resource = resources[random.nextInt(2)]; val sent = mutableListOf<ConnectionWorkerCommand>()
            val session = ConnectionAudioSession(resource, { sent.add(it); true }, Dispatchers.Unconfined)
            val route = audioRoute(resource); val target = checkNotNull(session.admitted(route))
            val claims = List(random.nextInt(16)) { checkNotNull(target.authority.retain()) }.shuffled(kotlin.random.Random(random.nextInt()))
            try {
                session.clear(); assertNull(target.authority.retain()); assertTrue(sent.isEmpty())
                session.closed(route.lease)
                claims.forEach { it.retired(ResourceRetirement.RETIRED); it.retired(ResourceRetirement.NOT_STARTED) }
                assertEquals(listOf(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)), sent)
            } finally { session.retire() }
        }
    }
    @Test fun t989SharedSessionDefaultDispatcherRetainsOriginalClaimAndDeliversBoundedProof() = runBlocking {
        val sent = java.util.concurrent.CopyOnWriteArrayList<ConnectionWorkerCommand>()
        val complete = CompletableDeferred<Unit>(); val route = audioRoute()
        val session = ConnectionMediaConsumerSession<LocalAudioRoute, AudioConsumerTarget>(
            ConnectionAudioConsumerCommands(ConnectionManagement.Resource.SPEAKERS),
            { sent.add(it); complete.complete(Unit); true },
        ) { metadata, authority -> AudioConsumerTarget(ConnectionAudioTarget.endpoint(metadata), authority) }
        try {
            val target = checkNotNull(session.admitted(route)); val claim = checkNotNull(target.authority.retain())
            session.clear(); assertFalse(claim.current()); assertFalse(complete.isCompleted)
            assertTrue(session.closed(route.lease)); assertFalse(complete.isCompleted)
            claim.retired(ResourceRetirement.RETIRED)
            withTimeout(2000) { complete.await(); while (session.admitted(route) != null) delay(1) }
            assertEquals(listOf(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)), sent)
        } finally { session.retire() }
    }

}
