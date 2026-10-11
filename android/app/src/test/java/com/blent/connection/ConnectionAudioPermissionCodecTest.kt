package com.blent.connection

import com.blent.MediaLocalConsent
import org.junit.Assert.*
import org.junit.Test

class ConnectionAudioPermissionCodecTest {
    private val microphone = ConnectionManagement.Resource.MICROPHONE
    private fun commands(): List<ConnectionWorkerCommand.AudioPermission> = (0..15).flatMap { bits ->
        val context = MediaLocalConsent(bits and 1 != 0, bits and 2 != 0, bits and 4 != 0, bits and 8 != 0)
        listOf(false, true).map { ConnectionWorkerCommand.AudioPermission(audioLease(microphone), it, context) }
    }
    private fun rejected(action: () -> Unit) { assertThrows(Exception::class.java) { action() } }
    @Test fun t997EveryPermissionContextAndGrantRoundTripsThroughFlatPrivateIpc() {
        for (command in commands()) {
            val bytes = ConnectionWorkerCodec.command(command)
            assertTrue(bytes.size <= 110); assertEquals(8, bytes[1].toInt())
            assertEquals(command, ConnectionWorkerCodec.command(bytes))
            for (size in 0 until bytes.size) rejected { ConnectionWorkerCodec.command(bytes.copyOf(size)) }
            rejected { ConnectionWorkerCodec.command(bytes + 0) }
        }
        for (required in listOf(false, true)) {
            val event = ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(audioLease(microphone), audioRoute(microphone).profile, required))
            val bytes = ConnectionWorkerCodec.event(event)
            assertTrue(bytes.size <= 110); assertWorkerMetadata(event, ConnectionWorkerCodec.event(bytes))
            for (size in 0 until bytes.size) rejected { ConnectionWorkerCodec.event(bytes.copyOf(size)) }
        }
    }
    @Test fun t997EveryInvalidBooleanOrPlaybackPermissionRecordRejects() {
        val command = ConnectionWorkerCodec.command(commands().first())
        val offered = ConnectionWorkerCodec.event(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(audioLease(microphone), audioRoute(microphone).profile, true)))
        for (value in 2..255) {
            for (index in command.size - 5 until command.size)
                rejected { ConnectionWorkerCodec.command(command.copyOf().also { it[index] = value.toByte() }) }
            rejected { ConnectionWorkerCodec.event(offered.copyOf().also { it[it.lastIndex] = value.toByte() }) }
        }
        rejected { ConnectionWorkerCodec.command(command.copyOf().also { it[2] = 4 }) }
        rejected { ConnectionWorkerCodec.event(offered.copyOf().also { it[2] = 4 }) }
        rejected { ConnectionWorkerCommand.AudioPermission(audioLease(), true, MediaLocalConsent(true, true, true, false)) }
        rejected { AudioPermissionRequest(audioLease()) }
    }
    @Test fun t997CounterBoundsAndSeededMutationsNeverCreateUncheckedMetadata() {
        val command = commands().first()
        for (value in listOf(Long.MIN_VALUE, -1L, 0L)) {
            val lease = command.lease
            val invalid = listOf(lease.copy(owner = value), lease.copy(connection = lease.connection.copy(epoch = value)),
                lease.copy(key = lease.key.copy(request = value)), lease.copy(key = lease.key.copy(generation = value)))
            for (bad in invalid) {
                rejected { AudioPermissionRequest(bad) }; rejected { command.copy(lease = bad) }
            }
        }
        val maximum = command.lease.let { it.copy(owner = Long.MAX_VALUE, connection = it.connection.copy(epoch = Long.MAX_VALUE),
            key = it.key.copy(request = Long.MAX_VALUE, generation = Long.MAX_VALUE)) }
        val edge = command.copy(lease = maximum)
        assertEquals(edge, ConnectionWorkerCodec.command(ConnectionWorkerCodec.command(edge)))
        val random = java.util.Random(997); val samples = commands()
        repeat(512) {
            val bytes = ConnectionWorkerCodec.command(samples[random.nextInt(samples.size)])
            bytes[random.nextInt(bytes.size)] = random.nextInt(256).toByte()
            runCatching { ConnectionWorkerCodec.command(bytes) }.getOrNull()?.let {
                assertArrayEquals(bytes, ConnectionWorkerCodec.command(it))
            }
        }
    }
}
