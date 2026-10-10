package com.blent.connection

import com.blent.connection.ConnectionManagement.Commit
import com.blent.connection.ConnectionManagement.Credential
import com.blent.connection.ConnectionManagement.Key
import com.blent.connection.ConnectionManagement.Message
import com.blent.connection.ConnectionManagement.Offer
import com.blent.connection.ConnectionManagement.Resource
import org.junit.Assert.*

/** Real host mux; test controls the physical write-completion boundary separately. */
internal class TabletSessionRig(quantum: Int = 65536, capabilities: Int = 15, timeout: Long = 100) {
    val ticket = ConnectionTicket(1, ConnectionPeer.parse("ab".repeat(32)))
    val tablet = TabletSession(ticket, ConnectionSessionLimits(4096, 64, quantum, timeout), capabilities)
    val host = ConnectionMux(1, 1, 4096, 64, quantum)
    val messages = ArrayList<Message>()
    val payloads = ArrayList<Pair<Int, ByteArray>>()
    private val decoder = ManagementDecoder()
    var now = 0L
    init { pump() }

    fun send(message: Message) { assertTrue(host.offer(6, ConnectionManagement.encode(message))); hostDrain() }
    fun hostStep(): Boolean {
        val frame = host.next() ?: return false
        tablet.receive(ConnectionFrame(frame.header, frame.bytes, frame.offset, frame.length), now)
        host.finish(true)
        return true
    }
    fun hostDrain() {
        repeat(4096) { if (!hostStep()) return }
        error("T882 host did not quiesce")
    }
    fun receive(frame: ConnectionFrame) {
        if (!host.receive(frame.header)) return
        if (frame.header.channel == 6) management(frame)
        else payloads.add(frame.header.channel to frame.bytes.copyOfRange(frame.offset, frame.offset + frame.length))
        host.release(frame.header.channel, frame.length)
    }
    private fun management(frame: ConnectionFrame) {
        var offset = frame.offset
        val end = offset + frame.length
        while (offset < end) {
            val result = decoder.feed(frame.bytes, offset, end - offset)
            offset += result.consumed
            result.message?.let(messages::add)
        }
    }
    fun tabletStep(): Boolean {
        val frame = tablet.nextFrame(now) ?: return false
        receive(frame)
        tablet.written(frame, now)
        return true
    }
    fun pump() {
        repeat(4096) {
            val outgoing = hostStep()
            val incoming = tabletStep()
            if (!outgoing && !incoming) return
        }
        error("T882 link did not quiesce")
    }
    fun invitation(resource: Resource = Resource.DISPLAY, request: Long = 1, generation: Long = 1): Offer {
        val profile = when (resource) {
            Resource.DISPLAY -> ConnectionManagement.Display
            Resource.CAMERA -> ConnectionManagement.Camera(640, 480, 30, 1000, 256, 100, 0, false, true)
            else -> ConnectionManagement.Audio(1, 40, false)
        }
        return Offer(Key(resource, request, generation), Credential(ByteArray(32)), profile)
    }
    fun offered(resource: Resource = Resource.DISPLAY, request: Long = 1, generation: Long = 1): ConnectionResourceLease {
        send(invitation(resource, request, generation))
        return (tablet.nextEvent() as ConnectionSessionEvent.Offered).lease
    }
    fun admit(resource: Resource = Resource.DISPLAY, request: Long = 1): ConnectionResourceLease {
        val lease = offered(resource, request)
        tablet.prepared(lease, now); pump()
        send(Commit(lease.key))
        ConnectionChannels.members(resource).forEach { host.activate(it, lease.key.generation) }
        pump()
        assertEquals(ConnectionSessionEvent.State(lease, ConnectionResourceState.ADMITTED), tablet.nextEvent())
        assertTrue(tablet.current(lease))
        return lease
    }
    fun events(): List<ConnectionSessionEvent> {
        val result = ArrayList<ConnectionSessionEvent>()
        repeat(64) { result.add(tablet.nextEvent() ?: return result) }
        error("T882 event queue unbounded")
    }
    fun peerClose(lease: ConnectionResourceLease) {
        for (channel in ConnectionChannels.members(lease.key.resource)) {
            if (host.acceptsData(channel, lease.key.generation)) host.close(channel, ConnectionWire.CLOSE)
            host.consumerRetired(channel, lease.key.generation)
        }
        hostDrain()
    }
}
