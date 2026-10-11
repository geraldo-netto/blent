package com.blent.connection

import com.blent.MediaLocalConsent
import org.junit.Assert.*
import org.junit.Test

internal fun audioLease(resource: ConnectionManagement.Resource = ConnectionManagement.Resource.SPEAKERS, owner: Long = 1) =
    cameraLease(owner).copy(key = cameraLease(owner).key.copy(resource = resource))
internal fun audioRoute(resource: ConnectionManagement.Resource = ConnectionManagement.Resource.SPEAKERS) =
    LocalAudioRoute(audioLease(resource), ConnectionManagement.Credential(ByteArray(32) { it.toByte() }), 19898, ConnectionManagement.Audio(2, 40, false))

class ConnectionAudioCodecTest {
    private val resources = listOf(ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS)
    private fun rejected(action: () -> Unit) { assertThrows(Exception::class.java) { action() } }
    private fun commands(): List<ConnectionWorkerCommand.Audio> = resources.flatMap { resource ->
        (0..15).map { flags -> ConnectionWorkerCommand.AudioChoice(audioLease(resource),
            MediaLocalConsent(flags and 1 != 0, flags and 2 != 0, flags and 4 != 0, flags and 8 != 0))
        } + ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resource))
    }
    private fun events(): List<ConnectionWorkerEvent.Audio> = resources.flatMap { resource ->
        val route = audioRoute(resource)
        listOf(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)),
            ConnectionWorkerEvent.AudioRoute(route), ConnectionWorkerEvent.AudioClosed(route.lease))
    }
    @Test fun t988BothDirectionsAndAllLocalContextsRoundTripThroughBoundedFlatMetadata() {
        for (command in commands()) {
            val record = ConnectionWorkerCodec.command(command)
            assertTrue(record.size <= 110); assertEquals(command, ConnectionWorkerCodec.command(record))
        }
        for (event in events()) {
            val record = ConnectionWorkerCodec.event(event)
            assertTrue(record.size <= 150); assertWorkerMetadata(event, ConnectionWorkerCodec.event(record))
        }
        assertEquals(6, ConnectionWorkerCodec.command(commands().first())[1].toInt())
        assertEquals(7, ConnectionWorkerCodec.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease()))[1].toInt())
        assertEquals(listOf(11, 12, 13), events().take(3).map { ConnectionWorkerCodec.event(it)[1].toInt() })
    }
    @Test fun t988EveryTruncationVersionTrailingBytesAndOversizeReject() {
        for (command in commands()) {
            val record = ConnectionWorkerCodec.command(command)
            for (size in 0 until record.size) rejected { ConnectionWorkerCodec.command(record.copyOf(size)) }
            rejected { ConnectionWorkerCodec.command(record + 0) }
            rejected { ConnectionWorkerCodec.command(record.copyOf().also { it[0] = 2 }) }
        }
        for (event in events()) {
            val record = ConnectionWorkerCodec.event(event)
            for (size in 0 until record.size) rejected { ConnectionWorkerCodec.event(record.copyOf(size)) }
            rejected { ConnectionWorkerCodec.event(record + 0) }
            rejected { ConnectionWorkerCodec.event(record.copyOf().also { it[0] = 2 }) }
        }
        rejected { ConnectionWorkerCodec.command(ByteArray(ConnectionWorkerCodec.LIMIT + 1)) }
        rejected { ConnectionWorkerCodec.event(ByteArray(ConnectionWorkerCodec.LIMIT + 1)) }
    }
    @Test fun t988AllInvalidResourceAndBooleanBytesRejectConstructedOrDecodedMetadata() {
        val command = ConnectionWorkerCodec.command(commands().first())
        val event = ConnectionWorkerCodec.event(events().first())
        for (value in 0..255) {
            if (value !in 3..4) {
                rejected { ConnectionWorkerCodec.command(command.copyOf().also { it[2] = value.toByte() }) }
                rejected { ConnectionWorkerCodec.event(event.copyOf().also { it[2] = value.toByte() }) }
            }
            if (value > 1) invalidBooleans(command, event, value)
        }
        for (resource in listOf(ConnectionManagement.Resource.DISPLAY, ConnectionManagement.Resource.CAMERA)) {
            val lease = audioLease(resource)
            rejected { ConnectionWorkerCommand.AudioChoice(lease, MediaLocalConsent(true, true, true, false)) }
            rejected { ConnectionWorkerCommand.AudioConsumerRetired(lease) }
            rejected { ConnectionWorkerEvent.AudioClosed(lease) }
        }
    }
    private fun invalidBooleans(command: ByteArray, event: ByteArray, value: Int) {
        for (index in command.size - 4 until command.size)
            rejected { ConnectionWorkerCodec.command(command.copyOf().also { it[index] = value.toByte() }) }
        for (index in event.size - 2 until event.size)
            rejected { ConnectionWorkerCodec.event(event.copyOf().also { it[index] = value.toByte() }) }
    }
    @Test fun t988LeaseProfileAndPortBoundariesUseCanonicalContracts() {
        for (resource in resources) {
            for (value in listOf(Long.MIN_VALUE, -1, 0)) invalidLease(audioLease(resource), value)
            for (port in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE)) rejected { audioRoute(resource).copy(port = port) }
            validProfiles(resource)
        }
        val offered = ConnectionWorkerCodec.event(events().first())
        for (processing in 0..255) if (processing !in 1..2)
            rejected { ConnectionWorkerCodec.event(offered.copyOf().also { it[it.size - 5] = processing.toByte() }) }
        for (buffer in 0..220) if (buffer !in 20..200 || buffer % 10 != 0)
            rejected { ConnectionWorkerCodec.event(offered.copyOf().also { it[it.size - 4] = 0; it[it.size - 3] = buffer.toByte() }) }
    }
    private fun invalidLease(lease: ConnectionResourceLease, value: Long) {
        val alternatives = listOf(lease.copy(owner = value), lease.copy(connection = lease.connection.copy(epoch = value)),
            lease.copy(key = lease.key.copy(request = value)), lease.copy(key = lease.key.copy(generation = value)))
        for (invalid in alternatives) {
            rejected { ConnectionWorkerCommand.AudioChoice(invalid, MediaLocalConsent(true, true, true, false)) }
            rejected { ConnectionWorkerCommand.AudioConsumerRetired(invalid) }
            rejected { ConnectionWorkerEvent.AudioClosed(invalid) }
        }
    }
    private fun validProfiles(resource: ConnectionManagement.Resource) {
        for (processing in 1..2) for (buffer in listOf(20, 200)) for (background in listOf(false, true)) for (port in listOf(1, 65535)) {
            val route = audioRoute(resource).copy(profile = ConnectionManagement.Audio(processing, buffer, background), port = port)
            val event = ConnectionWorkerEvent.AudioRoute(route)
            assertWorkerMetadata(event, ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(event)))
        }
    }
    @Test fun t988SeededMutationsAcceptOnlyValidatedCanonicalRecordsAndTagsStayDisjoint() {
        val random = java.util.Random(988); val commands = commands(); val events = events()
        repeat(1024) {
            val command = ConnectionWorkerCodec.command(commands[random.nextInt(commands.size)])
            command[random.nextInt(command.size)] = random.nextInt(256).toByte()
            runCatching { ConnectionWorkerCodec.command(command) }.getOrNull()?.let {
                assertArrayEquals(command, ConnectionWorkerCodec.command(it))
            }
            val event = ConnectionWorkerCodec.event(events[random.nextInt(events.size)])
            event[random.nextInt(event.size)] = random.nextInt(256).toByte()
            runCatching { ConnectionWorkerCodec.event(event) }.getOrNull()?.let {
                assertArrayEquals(event, ConnectionWorkerCodec.event(it))
            }
        }
        for (tag in 0..255) {
            if (tag !in 1..8) rejected { ConnectionWorkerCodec.command(byteArrayOf(1, tag.toByte())) }
            if (tag !in 1..13) rejected { ConnectionWorkerCodec.event(byteArrayOf(1, tag.toByte())) }
        }
    }
    @Test fun t988SignedLeaseCounterExtremesRoundTripOrRejectBeforeOriginalOwnership() {
        for (resource in resources) {
            val maximum = audioLease(resource, Long.MAX_VALUE).let { it.copy(connection = it.connection.copy(epoch = Long.MAX_VALUE)) }
            val admitted = ConnectionWorkerCommand.AudioChoice(maximum, MediaLocalConsent(true, true, true, true))
            assertEquals(admitted, ConnectionWorkerCodec.command(ConnectionWorkerCodec.command(admitted)))
            val bytes = ConnectionWorkerCodec.command(ConnectionWorkerCommand.AudioChoice(audioLease(resource), MediaLocalConsent(true, true, true, false)))
            // Prefix version/tag/resource, then epoch, UTF peer, and three lease counters.
            val owner = bytes.size - 4 - 24
            for (offset in listOf(3, owner, owner + 8, owner + 16)) for (value in listOf(Long.MIN_VALUE, -1, 0)) {
                val invalid = bytes.copyOf()
                java.nio.ByteBuffer.wrap(invalid).putLong(offset, value)
                rejected { ConnectionWorkerCodec.command(invalid) }
            }
        }
    }

    @Test fun t988ActorDoesNotClaimAudioSupportBeforeOriginalWorkerComposition() {
        val rig = RuntimeRig(); rig.start()
        assertFalse(rig.owner.command(ConnectionWorkerCommand.AudioChoice(audioLease(), MediaLocalConsent(true, true, true, false))))
        assertFalse(rig.owner.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease())))
        assertFalse(rig.run.stopped); assertEquals(1, rig.creates)
        val presentation = ConnectionPresentation(); val token = presentation.begin()
        presentation.event(token, ConnectionWorkerEvent.Peer(rig.setup.ticket, rig.setup.ticket.peer, false))
        presentation.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        presentation.event(token, ConnectionWorkerEvent.AudioRoute(audioRoute()))
        assertNull(presentation.state.value.camera); assertNull(presentation.state.value.display)
        assertTrue(presentation.state.value.authenticated)
    }
}
