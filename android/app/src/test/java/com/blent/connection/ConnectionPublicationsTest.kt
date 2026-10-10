package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionPublicationsTest {
    private val key = ConnectionManagement.Key(ConnectionManagement.Resource.DISPLAY, 1, 1)
    private fun lease() = ConnectionResourceLease(
        ConnectionTicket(1, ConnectionPeer.parse("ab".repeat(32))), key, 1)
    private fun mux(quantum: Int = 7) = ConnectionMux(1, 1, 4096, 16, quantum)
    private fun grant(owner: ConnectionMux) {
        val peer = mux()
        owner.receive(checkNotNull(peer.next()).header)
        peer.finish(true)
    }
    private fun drain(owner: ConnectionMux, publications: ConnectionPublications): List<ConnectionResourceLease> {
        val effects = ArrayList<ConnectionResourceLease>()
        repeat(128) {
            publications.publish(owner)
            val frame = owner.next() ?: return effects
            owner.finish(true)
            publications.written(frame.header)?.let(effects::add)
        }
        error("T882 publication failed to drain")
    }
    @Test fun t882AckEffectRequiresWholeRecordCompletion() {
        val owner = mux(); grant(owner)
        val publications = ConnectionPublications()
        val lease = lease()
        publications.queue(ConnectionManagement.ResultMessage(key, ConnectionManagement.Outcome.REVOKED), lease)
        publications.queue(ConnectionManagement.Capabilities(15))
        publications.publish(owner); publications.publish(owner)
        val credit = checkNotNull(owner.next()); owner.finish(true)
        assertNull(publications.written(credit.header))
        val first = checkNotNull(owner.next()); assertEquals(7, first.length)
        owner.finish(true); assertNull(publications.written(first.header))
        publications.publish(owner)
        assertEquals(listOf(lease), drain(owner, publications))
        assertNull(owner.next())
    }
    @Test fun t882PublicationCapacityCountsActiveUntilActualCompletion() {
        val owner = mux(); grant(owner)
        val publications = ConnectionPublications()
        repeat(16) { publications.queue(ConnectionManagement.Capabilities(it)) }
        publications.publish(owner)
        assertThrows(IllegalStateException::class.java) { publications.room() }
        assertThrows(IllegalStateException::class.java) { publications.queue(ConnectionManagement.Capabilities(0)) }
        assertEquals(emptyList<ConnectionResourceLease>(), drain(owner, publications))
        publications.room(16)
        publications.queue(ConnectionManagement.Capabilities(0)); publications.clear()
        publications.publish(owner); assertNull(owner.next()); publications.room(16)
    }
    @Test fun t882RejectsUnownedOrOutOfBoundsCompletions() {
        val publications = ConnectionPublications()
        val owner = mux(); grant(owner)
        val header = ConnectionWire.Header(ConnectionWire.DATA, 6, 1, 1, 0, 1, 0)
        assertThrows(IllegalStateException::class.java) { publications.written(header) }
        for (count in listOf(Int.MIN_VALUE, 0, 17, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { publications.room(count) }
        }
        publications.queue(ConnectionManagement.Capabilities(1)); publications.publish(owner)
        for (length in listOf(Int.MIN_VALUE, -1, 0, 30, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { publications.written(header.copy(length = length)) }
        }
        assertNull(publications.written(header.copy(channel = 1)))
        publications.clear(); assertThrows(IllegalStateException::class.java) { publications.written(header) }
    }
    @Test fun t882RejectsLostMuxQueueOwnership() {
        val owner = mux(); owner.close(6, ConnectionWire.CLOSE)
        val publications = ConnectionPublications(); publications.queue(ConnectionManagement.Capabilities(1))
        assertThrows(IllegalStateException::class.java) { publications.publish(owner) }
        publications.clear(); publications.publish(owner); publications.room(16)
    }
}
