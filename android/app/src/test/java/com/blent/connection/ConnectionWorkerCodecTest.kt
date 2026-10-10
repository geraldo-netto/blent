package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.util.Random

class ConnectionWorkerCodecTest {
    private val peer = ConnectionPeer.parse("ab".repeat(32))
    private val ticket = ConnectionTicket(7, peer)
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    @Test fun t884PrivateControlRoundTripsExactAuthorityAndLimits() {
        val spec = ConnectionWorkerSpec("usb-device", ConnectionSessionLimits(4096, 1, 64, 1), ConnectionRunLimits(1, 60000), 600000)
        assertEquals(spec, ConnectionWorkerCodec.spec(ConnectionWorkerCodec.spec(spec)))
        for (approve in listOf(false, true)) for (remember in listOf(false, true)) {
            val command = ConnectionWorkerCommand.Decide(ticket, approve, remember)
            assertEquals(command, ConnectionWorkerCodec.command(ConnectionWorkerCodec.command(command)))
        }
        val revoke = ConnectionWorkerCommand.Revoke(peer)
        assertEquals(revoke, ConnectionWorkerCodec.command(ConnectionWorkerCodec.command(revoke)))
    }
    @Test fun t854ProductionBoundsRoundTripAndRejectInvalidConfiguration() {
        for (edge in listOf(1, 1024)) {
            val spec = ConnectionWorkerSpec("usb", nativeTimeoutMs = 60000,
                trustedPeers = edge, accessoryCapacity = edge, timerMs = 1000,
                io = ConnectionRunLimits(123, 456))
            assertEquals(spec, ConnectionWorkerCodec.spec(ConnectionWorkerCodec.spec(spec)))
            assertEquals(ConnectionSetupLimits(262144, 120000, 60000, 456, edge), spec.setupLimits())
            rejected { ConnectionWorkerCodec.spec(ConnectionWorkerCodec.spec(spec).also { it[0] = 1 }) }
        }
        for (value in listOf(Int.MIN_VALUE, -1, 0, 1025, Int.MAX_VALUE)) {
            rejected { ConnectionWorkerSpec("usb", trustedPeers = value) }
            rejected { ConnectionWorkerSpec("usb", accessoryCapacity = value) }
        }
        for (value in listOf(Long.MIN_VALUE, -1, 0, 1001, Long.MAX_VALUE)) {
            rejected { ConnectionWorkerSpec("usb", timerMs = value) }
        }
        for (value in listOf(Long.MIN_VALUE, -1, 0, 60001, Long.MAX_VALUE)) {
            rejected { ConnectionWorkerSpec("usb", nativeTimeoutMs = value) }
        }
        val random = Random(854)
        val original = ConnectionWorkerCodec.spec(ConnectionWorkerSpec("usb"))
        repeat(1024) {
            val bytes = original.copyOf()
            bytes[random.nextInt(bytes.size)] = random.nextInt(256).toByte()
            val decoded = runCatching { ConnectionWorkerCodec.spec(bytes) }.getOrNull()
            if (decoded != null) assertEquals(decoded, ConnectionWorkerCodec.spec(ConnectionWorkerCodec.spec(decoded)))
        }
    }
    private fun events(): List<ConnectionWorkerEvent> = listOf(
        ConnectionWorkerEvent.Pairing(ticket, ConnectionPeer.parse("cd".repeat(32))),
        ConnectionWorkerEvent.Peer(ticket, peer, true), ConnectionWorkerEvent.Peer(ticket, peer, false),
        ConnectionWorkerEvent.Trust(peer, true), ConnectionWorkerEvent.Trust(peer, false),
        ConnectionWorkerEvent.Ended(null), ConnectionWorkerEvent.Ended("native close failed"),
    ) + ConnectionWorkerPhase.entries.map { ConnectionWorkerEvent.State(it) }
    @Test fun t884EventSchemaCarriesMetadataWithoutMediaArrays() {
        for (event in events()) assertEquals(event, ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(event)))
        assertArrayEquals(byteArrayOf(1, 2, 0), ConnectionWorkerCodec.event(ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING)))
        assertArrayEquals(byteArrayOf(1, 4, 0), ConnectionWorkerCodec.event(ConnectionWorkerEvent.Ended(null)))
        val text = "\u2603".repeat(512)
        assertEquals(ConnectionWorkerSpec(text), ConnectionWorkerCodec.spec(ConnectionWorkerCodec.spec(ConnectionWorkerSpec(text))))
        assertEquals(ConnectionWorkerEvent.Ended(text), ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(ConnectionWorkerEvent.Ended(text))))
    }
    @Test fun t884MalformedTruncatedAndTrailingRecordsNeverReachControl() {
        val records = listOf(
            ConnectionWorkerCodec.spec(ConnectionWorkerSpec("usb")) to { b: ByteArray -> ConnectionWorkerCodec.spec(b) },
            ConnectionWorkerCodec.command(ConnectionWorkerCommand.Decide(ticket, true, false)) to { b: ByteArray -> ConnectionWorkerCodec.command(b) },
            ConnectionWorkerCodec.event(ConnectionWorkerEvent.Pairing(ticket, peer)) to { b: ByteArray -> ConnectionWorkerCodec.event(b) },
        )
        for ((bytes, decode) in records) {
            for (length in 0 until bytes.size) rejected { decode(bytes.copyOf(length)) }
            rejected { decode(bytes + 0) }; rejected { decode(bytes.copyOf().also { it[0] = 127 }) }
            rejected { decode(ByteArray(ConnectionWorkerCodec.LIMIT + 1)) }
        }
    }
    @Test fun t884ExtremeSpecTicketStringAndBooleanValuesReject() {
        for (device in listOf("", "x".repeat(513), "a\u0000b")) rejected { ConnectionWorkerSpec(device) }
        for (timeout in listOf(Long.MIN_VALUE, -1, 0, 600001, Long.MAX_VALUE)) {
            rejected { ConnectionWorkerSpec("usb", approvalTimeoutMs = timeout) }
        }
        for (epoch in listOf(Long.MIN_VALUE, -1, 0)) {
            rejected { ConnectionWorkerCodec.command(ConnectionWorkerCommand.Decide(ticket.copy(epoch = epoch), true, false)) }
        }
        for (text in listOf("x".repeat(513), "a\u0000b")) rejected { ConnectionWorkerCodec.event(ConnectionWorkerEvent.Ended(text)) }
        for (byte in listOf(2, 127, 255)) {
            rejected { ConnectionWorkerCodec.event(byteArrayOf(1, 4, byte.toByte())) }
            rejected { ConnectionWorkerCodec.event(byteArrayOf(1, 2, 127)) }
            rejected { ConnectionWorkerCodec.command(byteArrayOf(1, byte.toByte())) }
        }
    }
    @Test fun t895PeerMetadataRejectsInvalidBooleanAndRoundTripsMutations() {
        val original = ConnectionWorkerCodec.event(ConnectionWorkerEvent.Peer(ticket, peer, true))
        for (value in listOf(2, 127, 255)) {
            rejected { ConnectionWorkerCodec.event(original.copyOf().also { it[it.lastIndex] = value.toByte() }) }
        }
        val random = Random(895)
        repeat(1024) {
            val bytes = original.copyOf()
            bytes[random.nextInt(bytes.size)] = random.nextInt(256).toByte()
            val decoded = runCatching { ConnectionWorkerCodec.event(bytes) }.getOrNull()
            if (decoded != null) assertEquals(decoded, ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(decoded)))
        }
    }
    @Test fun t884SeededBoundedMutationCannotEscapeFlatSchema() {
        val random = Random(884)
        val original = ConnectionWorkerCodec.command(ConnectionWorkerCommand.Decide(ticket, true, false))
        repeat(1024) {
            val bytes = original.copyOf()
            bytes[random.nextInt(bytes.size)] = random.nextInt(256).toByte()
            val decoded = runCatching { ConnectionWorkerCodec.command(bytes) }.getOrNull()
            if (decoded != null) assertEquals(decoded, ConnectionWorkerCodec.command(ConnectionWorkerCodec.command(decoded)))
        }
        for (record in events()) {
            val bytes = ConnectionWorkerCodec.event(record)
            for (length in 0 until bytes.size) rejected { ConnectionWorkerCodec.event(bytes.copyOf(length)) }
        }
    }
}
