package com.blent.connection

import com.blent.connection.ConnectionManagement.Key
import com.blent.connection.ConnectionManagement.Outcome
import com.blent.connection.ConnectionManagement.Resource
import com.blent.connection.ConnectionResources.Install
import com.blent.connection.ConnectionResources.Lease
import com.blent.connection.ConnectionResources.Role
import org.junit.Assert.*
import org.junit.Test

class ConnectionResourcesTest {
    private fun connection(epoch: Long = 1) = ConnectionPairing.Ticket(epoch, ConnectionPeer.parse("ab".repeat(32)))
    private fun pair() = ConnectionResources(Role.HOST, connection()) to ConnectionResources(Role.TABLET, connection())
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    private fun offer(h: ConnectionResources, t: ConnectionResources, resource: Resource): Pair<Lease, Lease> {
        val a = h.begin(resource, 11, 0, 100)
        val b = t.offer(a.key, 22, 0, 100)
        assertEquals(11L, a.owner); assertEquals(22L, b.owner)
        assertEquals(connection(), a.connection); assertEquals(connection(), b.connection)
        return a to b
    }
    private fun committed(h: ConnectionResources, t: ConnectionResources, resource: Resource): Pair<Lease, Lease> {
        val (a, b) = offer(h, t, resource)
        assertTrue(t.prepared(b, 1)); assertTrue(h.prepared(a, 2)); h.startCommit(a, 3)
        assertEquals(Install.ACTIVATE, h.commit(a, 4)); assertEquals(Install.ACTIVATE, t.commit(b, 5))
        return a to b
    }
    private fun active(h: ConnectionResources, t: ConnectionResources, resource: Resource): Pair<Lease, Lease> {
        val (a, b) = committed(h, t, resource)
        assertTrue(t.admitted(b, 6)); assertTrue(h.admitted(a, 7))
        assertTrue(h.current(a)); assertTrue(t.current(b)); return a to b
    }
    private fun finish(h: ConnectionResources, t: ConnectionResources, a: Lease, b: Lease, installed: Boolean) {
        h.revoke(a); h.revokeWritten(a); t.peerRevoke(b); rejected { t.acknowledge(b) }
        t.nativeRetired(b); if (installed) t.channelsRetired(b)
        assertTrue(t.readyToAck(b)); t.acknowledge(b); h.acknowledge(a)
        rejected { h.finish(a) }; h.nativeRetired(a); if (installed) h.channelsRetired(a)
        t.finish(b); h.finish(a)
    }
    @Test fun t851IndependentResourcesRevokeBeforeRetirementAndReuse() {
        val (h, t) = pair(); val (display, displayT) = active(h, t, Resource.DISPLAY)
        val (camera, cameraT) = active(h, t, Resource.CAMERA)
        rejected { h.begin(Resource.CAMERA, 11, 0, 100) }
        assertTrue(h.revoke(camera)); assertFalse(h.revoke(camera)); assertFalse(h.current(camera))
        assertTrue(h.current(display)); assertTrue(t.current(displayT))
        finish(h, t, camera, cameraT, true)
        val (next, nextT) = offer(h, t, Resource.CAMERA)
        assertEquals(camera.key.generation + 1, next.key.generation)
        assertFalse(h.current(camera)); rejected { h.nativeRetired(camera) }
        finish(h, t, next, nextT, false); finish(h, t, display, displayT, true)
        assertFalse(h.current(display)); rejected { h.prepared(display, 1) }
    }
    @Test fun t851CancelCrossingCommitInstallsDrainWithoutAuthority() {
        val (h, t) = pair(); val (a, b) = offer(h, t, Resource.CAMERA)
        t.prepared(b, 1); h.prepared(a, 2); h.startCommit(a, 3)
        t.revoke(b); h.peerRevoke(a); rejected { h.revokeWritten(a) }
        assertEquals(Install.DRAIN, h.commit(a, 4)); assertEquals(Install.DRAIN, t.commit(b, 5))
        rejected { t.admitted(b, 6) }; assertFalse(h.current(a)); finish(h, t, a, b, true)
    }
    @Test fun t851DenialAndStaleConsentNeverConsumeGeneration() {
        val (h, t) = pair()
        for (outcome in listOf(Outcome.UNAVAILABLE, Outcome.PERMISSION_DENIED, Outcome.BUSY, Outcome.FAILED)) {
            val (a, b) = offer(h, t, Resource.MICROPHONE)
            assertTrue(t.permissionRequired(b, 1)); assertTrue(h.permissionRequired(a, 2))
            rejected { t.permissionRequired(b, 3) }; t.reject(b, outcome); h.reject(a, outcome)
            rejected { t.prepared(b, 4) }; rejected { h.startCommit(a, 4) }
            finish(h, t, a, b, false); assertEquals(1L, a.key.generation)
        }
    }
    @Test fun t851AbsoluteDeadlineAndDisconnectDoNotProveRetirement() {
        val (h, t) = pair(); val (a, b) = offer(h, t, Resource.SPEAKERS)
        t.prepared(b, 1); h.prepared(a, 2); h.startCommit(a, 3)
        assertTrue(h.expire(99).all { it == null }); assertEquals(a, h.expire(100)[3])
        assertTrue(h.expire(100).all { it == null })
        assertEquals(Install.DRAIN, h.commit(a, 101)); assertEquals(Install.DRAIN, t.commit(b, 100))
        rejected { t.prepared(b, 101) }; h.disconnect(); t.disconnect()
        rejected { h.finish(a) }; rejected { t.finish(b) }
        h.nativeRetired(a); t.nativeRetired(b); rejected { h.finish(a) }
        h.channelsRetired(a); t.channelsRetired(b); h.finish(a); t.finish(b)
        rejected { h.begin(Resource.SPEAKERS, 11, 0, 100) }
        val next = ConnectionResources(Role.HOST, connection(2)); val fresh = next.begin(Resource.SPEAKERS, 11, 0, 100)
        assertNotEquals(fresh, a); rejected { next.prepared(a, 1) }
    }
    @Test fun t851InvalidBoundsLeaveAdmissionUnchanged() {
        for (epoch in listOf(Long.MIN_VALUE, 0L)) rejected { ConnectionResources(Role.HOST, connection(epoch)) }
        val (h, t) = pair()
        for (owner in listOf(Long.MIN_VALUE, 0L)) rejected { h.begin(Resource.DISPLAY, owner, 0, 100) }
        for (now in listOf(-1L, Long.MIN_VALUE, Long.MAX_VALUE)) rejected { h.begin(Resource.DISPLAY, 11, now, 100) }
        for (timeout in listOf(-1L, 0L, 600001L, Long.MAX_VALUE)) rejected { h.begin(Resource.DISPLAY, 11, 0, timeout) }
        rejected { h.expire(-1) }; rejected { t.begin(Resource.DISPLAY, 22, 0, 100) }
        val key = Key(Resource.DISPLAY, 1, 1)
        rejected { h.offer(key, 11, 0, 100) }
        for (invalid in listOf(key.copy(request = 0), key.copy(generation = 0), key.copy(generation = 2))) rejected { t.offer(invalid, 22, 0, 100) }
        val (a, b) = offer(h, t, Resource.DISPLAY); assertEquals(1L, a.key.request)
        finish(h, t, a, b, false); rejected { t.offer(key, 22, 0, 100) }
    }
    @Test fun t851WrongOrderAndStaleCallbacksNeverGainAuthority() {
        val (h, t) = pair(); val (a, b) = offer(h, t, Resource.CAMERA)
        assertFalse(h.readyToAck(a)); assertFalse(t.readyToAck(b))
        rejected { t.startCommit(b, 1) }; rejected { h.commit(a, 1) }; rejected { t.admitted(b, 1) }
        rejected { h.nativeRetired(a) }; rejected { h.channelsRetired(a) }; rejected { t.revokeWritten(b) }
        for (outcome in listOf(Outcome.ADMITTED, Outcome.PERMISSION_REQUIRED, Outcome.REVOKED)) rejected { t.reject(b, outcome) }
        for (changed in listOf(a.copy(owner = 12), a.copy(connection = connection(2)), a.copy(key = a.key.copy(request = 2)))) {
            assertFalse(h.current(changed)); rejected { h.prepared(changed, 1) }
        }
        t.prepared(b, 1); h.prepared(a, 2); rejected { t.prepared(b, 2) }; rejected { t.reject(b, Outcome.BUSY) }
        h.revoke(a); h.revokeWritten(a); rejected { h.revokeWritten(a) }; t.peerRevoke(b)
        rejected { t.peerRevoke(b) }; rejected { t.commit(b, 3) }
        t.nativeRetired(b); rejected { t.nativeRetired(b) }; rejected { t.channelsRetired(b) }
        t.acknowledge(b); rejected { t.acknowledge(b) }; h.acknowledge(a); h.nativeRetired(a)
        t.finish(b); h.finish(a)
    }
    @Test fun t851LateResultsAndRepeatedBoundedSessionsStayIsolated() {
        val (h, t) = pair()
        repeat(1024) { iteration ->
            val resource = Resource.entries[iteration % 4]; val (a, b) = offer(h, t, resource)
            t.permissionRequired(b, 1); assertFalse(h.permissionRequired(a, 100))
            assertFalse(h.prepared(a, 101)); rejected { t.permissionRequired(b, 100) }
            finish(h, t, a, b, false)
        }
        val (a, b) = committed(h, t, Resource.DISPLAY)
        assertTrue(t.admitted(b, 6)); assertFalse(h.admitted(a, 100))
        assertTrue(t.expire(Long.MAX_VALUE).all { it == null }); finish(h, t, a, b, true)
    }
    @Test fun t851InvalidTimesNeverConsumeATransition() {
        val (h, t) = pair(); val (a, b) = offer(h, t, Resource.CAMERA)
        for (time in listOf(Long.MIN_VALUE, -1L)) {
            rejected { h.permissionRequired(a, time) }; rejected { t.permissionRequired(b, time) }
            rejected { h.prepared(a, time) }; rejected { t.prepared(b, time) }
        }
        t.prepared(b, 1); h.prepared(a, 2); rejected { h.startCommit(a, -1) }; h.startCommit(a, 3)
        rejected { h.commit(a, -1) }; rejected { t.commit(b, -1) }
        h.commit(a, 4); t.commit(b, 5)
        rejected { h.admitted(a, -1) }; rejected { t.admitted(b, -1) }
        t.admitted(b, 6); h.admitted(a, 7); finish(h, t, a, b, true)
    }
    private fun transfer(source: ConnectionMux, target: ConnectionMux): Boolean {
        val frame = source.next() ?: return false
        val delivered = target.receive(frame.header)
        if (frame.header.kind == ConnectionWire.DATA) {
            assertTrue(delivered); assertEquals(2, frame.header.channel)
            assertArrayEquals(byteArrayOf(42), frame.bytes.copyOfRange(frame.offset, frame.offset + frame.length))
        }
        source.finish(true)
        if (delivered) target.release(frame.header.channel, frame.header.length)
        return true
    }
    private fun exchange(left: ConnectionMux, right: ConnectionMux) {
        repeat(24) {
            val movedLeft = transfer(left, right); val movedRight = transfer(right, left)
            if (!movedLeft && !movedRight) return
        }
        fail("Bounded metadata exchange did not settle")
    }
    @Test fun t851CrossingCommitDrainsRealCreditsAndPreservesDisplay() {
        val (h, t) = pair(); val left = ConnectionMux(1, 1, 64, 4, 64); val right = ConnectionMux(1, 1, 64, 4, 64)
        left.activate(2, 1); right.activate(2, 1); exchange(left, right)
        val (a, b) = offer(h, t, Resource.CAMERA)
        t.prepared(b, 1); h.prepared(a, 2); h.startCommit(a, 3); t.revoke(b)
        assertEquals(Install.ACTIVATE, h.commit(a, 4)); left.activate(3, a.key.generation)
        assertEquals(Install.DRAIN, t.commit(b, 5)); right.close(3, ConnectionWire.RESET)
        assertFalse(right.offer(3, byteArrayOf(1)))
        val credit = checkNotNull(left.next()).header
        assertEquals(ConnectionWire.CREDIT, credit.kind); assertEquals(3, credit.channel)
        assertFalse(right.receive(credit)); left.finish(true)
        h.peerRevoke(a); left.close(3, ConnectionWire.RESET); h.revokeWritten(a); t.peerRevoke(b)
        exchange(left, right); assertFalse(left.retired(3)); assertFalse(right.retired(3))
        assertTrue(left.offer(2, byteArrayOf(42))); exchange(left, right)
        left.consumerRetired(3, 1); right.consumerRetired(3, 1)
        assertTrue(left.retired(3)); assertTrue(right.retired(3))
        h.channelsRetired(a); t.channelsRetired(b); t.nativeRetired(b); t.acknowledge(b)
        h.acknowledge(a); h.nativeRetired(a); h.finish(a); t.finish(b)
        val (replacement, _) = offer(h, t, Resource.CAMERA); assertEquals(2L, replacement.key.generation)
        left.replaceChannel(3, 2); right.replaceChannel(3, 2)
        assertFalse(left.offer(3, byteArrayOf(1))); assertTrue(left.offer(2, byteArrayOf(42))); exchange(left, right)
    }
    @Test fun t851CounterExhaustionNeverWraps() {
        val (h, t) = pair()
        val request = ConnectionResources::class.java.getDeclaredField("request").apply { isAccessible = true }
        request.setLong(h, Long.MAX_VALUE); rejected { h.begin(Resource.DISPLAY, 11, 0, 100) }
        request.setLong(h, Long.MAX_VALUE - 1)
        val generations = ConnectionResources::class.java.getDeclaredField("generations").apply { isAccessible = true }
        (generations.get(h) as LongArray)[0] = Long.MAX_VALUE
        (generations.get(t) as LongArray)[0] = Long.MAX_VALUE
        val (a, b) = active(h, t, Resource.DISPLAY)
        assertEquals(Long.MAX_VALUE, a.key.request); assertEquals(Long.MAX_VALUE, a.key.generation)
        finish(h, t, a, b, true); request.setLong(h, 0)
        rejected { h.begin(Resource.DISPLAY, 11, 0, 100) }
        rejected { t.offer(Key(Resource.DISPLAY, Long.MAX_VALUE, Long.MAX_VALUE), 22, 0, 100) }
    }
}
