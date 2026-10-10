package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionTrustAdmissionTest {
    private fun peer(value: Char) = ConnectionPeer.parse(value.toString().repeat(64))
    private class Store : PeerTrustStore {
        var bytes: ByteArray? = null
        var fail = false
        override fun load() = bytes?.copyOf()
        override fun replace(record: ByteArray) { check(!fail); bytes = record.copyOf() }
        override fun close() {}
    }
    private fun rejected(body: () -> Unit) { assertThrows(Exception::class.java, body) }

    @Test fun t852RememberedAdmissionRequiresFreshProofAndActualRetirement() {
        val store = Store(); val trust = ConnectionTrust(store, peer('a'))
        val pairing = ConnectionPairing()
        val first = trust.begin(pairing, peer('b'), 0, 100)
        assertEquals(ConnectionPairing.Phase.PENDING, pairing.phase)
        pairing.approve(first, true, 1)
        val lease = trust.authenticated(pairing, first, peer('b'), 2)
        assertTrue(pairing.current(lease)); assertTrue(trust.trusted(peer('b')))
        pairing.cancel(first)
        rejected { trust.begin(pairing, peer('b'), 3, 100) }
        pairing.retired(first); trust.close()
        val reopened = ConnectionTrust(store, peer('a'))
        val next = reopened.begin(pairing, peer('b'), 4, 100)
        assertEquals(ConnectionPairing.Phase.APPROVED, pairing.phase)
        assertFalse(pairing.current(lease))
        val fresh = reopened.authenticated(pairing, next, peer('b'), 5)
        assertFalse(fresh.remember); assertTrue(pairing.current(fresh))
        reopened.revoke(pairing, peer('b'))
        assertFalse(pairing.current(fresh)); pairing.retired(next)
        assertEquals(ConnectionPairing.Phase.PENDING, pairing.apply { reopened.begin(this, peer('b'), 6, 100) }.phase)
    }

    @Test fun t852FailedRememberCannotLeaveAnAuthenticatedLease() {
        val store = Store(); val trust = ConnectionTrust(store, peer('a')); val pairing = ConnectionPairing()
        val ticket = trust.begin(pairing, peer('b'), 0, 10)
        pairing.approve(ticket, true, 1); store.fail = true
        rejected { trust.authenticated(pairing, ticket, peer('b'), 2) }
        assertEquals(ConnectionPairing.Phase.CLOSING, pairing.phase)
        assertFalse(pairing.current(ConnectionPairing.Lease(ticket, true)))
        assertFalse(trust.trusted(peer('b'))); assertNull(store.bytes)
        rejected { trust.begin(ConnectionPairing(), peer('b'), 3, 10) }
        pairing.retired(ticket)
        rejected { trust.begin(pairing, peer('b'), 3, 10) }
        assertEquals(ConnectionPairing.Phase.IDLE, pairing.phase)
    }

    @Test fun t852StaleAndIncorrectProofCannotCancelAnotherAttempt() {
        val trust = ConnectionTrust(Store(), peer('a')); val pairing = ConnectionPairing()
        val old = trust.begin(pairing, peer('b'), 0, 10)
        pairing.cancel(old); pairing.retired(old)
        val ticket = trust.begin(pairing, peer('c'), 1, 10)
        pairing.approve(ticket, false, 2)
        rejected { trust.authenticated(pairing, old, peer('b'), 3) }
        assertEquals(ConnectionPairing.Phase.APPROVED, pairing.phase)
        rejected { trust.authenticated(pairing, ticket, peer('b'), 3) }
        assertEquals(ConnectionPairing.Phase.CLOSING, pairing.phase)
        pairing.retired(ticket)
        val fresh = trust.begin(pairing, peer('c'), 4, 10)
        pairing.approve(fresh, false, 5)
        val lease = trust.authenticated(pairing, fresh, peer('c'), 6)
        assertTrue(pairing.current(lease)); assertFalse(trust.trusted(peer('c')))
        trust.close()
        rejected { trust.begin(ConnectionPairing(), peer('d'), 7, 10) }
    }

    @Test fun t852SharedTrustFailureBeforeProofClosesOnlyMatchingPendingConnection() {
        val store = Store(); val trust = ConnectionTrust(store, peer('a'))
        val first = ConnectionPairing(); val second = ConnectionPairing()
        val a = trust.begin(first, peer('b'), 0, 10); first.approve(a, true, 1)
        val b = trust.begin(second, peer('c'), 0, 10); second.approve(b, false, 1)
        store.fail = true
        rejected { trust.authenticated(first, a, peer('b'), 2) }
        rejected { trust.authenticated(second, b, peer('c'), 2) }
        assertEquals(ConnectionPairing.Phase.CLOSING, second.phase)
        assertFalse(second.current(ConnectionPairing.Lease(b, false)))
    }
    @Test fun t852BoundedInvalidAdmissionValuesNeverPublishTrust() {
        val trust = ConnectionTrust(Store(), peer('a'))
        for ((now, timeout) in listOf(-1L to 1L, 0L to 0L, 0L to 600001L, Long.MAX_VALUE to 1L)) {
            val pairing = ConnectionPairing()
            rejected { trust.begin(pairing, peer('b'), now, timeout) }
            assertEquals(ConnectionPairing.Phase.IDLE, pairing.phase)
        }
        repeat(256) { index ->
            val pairing = ConnectionPairing()
            val ticket = trust.begin(pairing, peer('b'), 0, 10)
            pairing.approve(ticket, true, 1)
            val selected = if (index % 2 == 0) ticket else ticket.copy(epoch = index.toLong() + 2)
            val now = if (index % 3 == 0) -1L else Long.MAX_VALUE
            rejected { trust.authenticated(pairing, selected, peer('b'), now) }
            val expected = if (selected == ticket) ConnectionPairing.Phase.CLOSING else ConnectionPairing.Phase.APPROVED
            assertEquals(expected, pairing.phase)
            assertFalse(trust.trusted(peer('b')))
        }
    }

}
