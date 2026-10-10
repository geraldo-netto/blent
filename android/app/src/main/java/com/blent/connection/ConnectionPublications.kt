package com.blent.connection

import java.util.ArrayDeque

/** Bounded FIFO of complete management records. Only one record is admitted to
 * the mux at a time, so whole-record completion can be correlated without
 * copying its payload. A returned lease is an ACK publication effect, not proof
 * of local/native retirement; the owning session checks that separately. */
internal class ConnectionPublications {
    private class Pending(val bytes: ByteArray, val acknowledged: ConnectionResourceLease?) {
        var remaining = bytes.size
    }
    private val waiting = ArrayDeque<Pending>()
    private var active: Pending? = null
    fun room(count: Int = 1) {
        require(count in 1..16)
        check(waiting.size + (if (active == null) 0 else 1) <= 16 - count) { "Management output full" }
    }
    fun queue(message: ConnectionManagement.Message, acknowledged: ConnectionResourceLease? = null) {
        room()
        waiting.addLast(Pending(ConnectionManagement.encode(message), acknowledged))
    }
    fun publish(mux: ConnectionMux) {
        if (active != null) return
        val next = waiting.peekFirst() ?: return
        check(mux.offer(ConnectionWire.MANAGEMENT, next.bytes)) { "Management queue ownership lost" }
        active = waiting.removeFirst()
    }
    fun written(header: ConnectionWire.Header): ConnectionResourceLease? {
        if (header.channel != ConnectionWire.MANAGEMENT || header.kind != ConnectionWire.DATA) return null
        val publication = checkNotNull(active) { "Unowned management completion" }
        require(header.length in 1..publication.remaining)
        publication.remaining -= header.length
        if (publication.remaining != 0) return null
        active = null
        return publication.acknowledged
    }
    fun clear() { waiting.clear(); active = null }
}
