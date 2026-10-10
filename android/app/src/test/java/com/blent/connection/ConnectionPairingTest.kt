package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionPairingTest {
    private fun peer(char: Char) = ConnectionPeer.parse(char.toString().repeat(64))
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    @Test fun t846ApprovalRequiresCurrentTlsProofAndNativeRetirement() {
        val owner = ConnectionPairing(); assertEquals(ConnectionPairing.Phase.IDLE, owner.phase)
        val ticket = owner.begin(peer('a'), false, 10, 100)
        assertEquals(ConnectionPairing.Phase.PENDING, owner.phase)
        rejected { owner.begin(peer('b'), true, 10, 100) }; rejected { owner.authenticated(ticket, peer('a'), 11) }
        rejected { owner.approve(ticket.copy(epoch = 99), true, 11) }
        owner.approve(ticket, true, 11); assertEquals(ConnectionPairing.Phase.APPROVED, owner.phase)
        rejected { owner.authenticated(ticket, peer('b'), 12) }
        val lease = owner.authenticated(ticket, peer('a'), 12); assertTrue(lease.remember); assertTrue(owner.current(lease)); assertFalse(owner.current(lease.copy(remember = false)))
        rejected { owner.authenticated(ticket, peer('a'), 13) }; assertFalse(owner.expire(10000)); assertFalse(owner.revoke(peer('b')))
        rejected { owner.retired(ticket) }; assertTrue(owner.revoke(peer('a'))); assertFalse(owner.current(lease))
        assertEquals(ConnectionPairing.Phase.CLOSING, owner.phase); rejected { owner.begin(peer('a'), true, 20, 100) }
        rejected { owner.retired(ticket.copy(epoch = 99)) }; owner.retired(ticket)
        assertFalse(owner.revoke(peer('a'))); assertFalse(owner.cancel(ticket))
        val replacement = owner.begin(peer('a'), true, 20, 100); assertNotEquals(replacement, ticket)
        assertFalse(owner.cancel(ticket)); rejected { owner.approve(ticket, true, 21) }; assertFalse(owner.current(lease))
        val next = owner.authenticated(replacement, peer('a'), 22); assertFalse(next.remember); assertTrue(owner.current(next))
    }
    @Test fun t846TimeoutBoundariesAndStaleCallbackFuzz() {
        val owner = ConnectionPairing(); assertFalse(owner.expire(0))
        for ((now, timeout) in listOf(-1L to 1L, 0L to 0L, 0L to -1L, 0L to 600001L, Long.MAX_VALUE to 1L)) {
            rejected { owner.begin(peer('a'), false, now, timeout) }
        }
        rejected { owner.expire(-1) }
        repeat(1024) { round ->
            val ticket = owner.begin(peer('a'), round % 2 == 0, 10, 2); assertFalse(owner.expire(11))
            rejected { owner.approve(ticket, true, -1) }; rejected { owner.authenticated(ticket, peer('a'), -1) }
            rejected { owner.approve(ticket, true, 12) }; rejected { owner.authenticated(ticket, peer('a'), 12) }
            assertTrue(owner.expire(12)); assertEquals(ConnectionPairing.Phase.CLOSING, owner.phase)
            assertFalse(owner.expire(13)); rejected { owner.authenticated(ticket, peer('a'), 11) }
            owner.retired(ticket); assertFalse(owner.cancel(ticket))
        }
    }
    @Test fun t846FingerprintBoundariesAndMutations() {
        assertEquals("f".repeat(64), peer('f').fingerprint)
        for (size in listOf(0, 1, 63, 65, 4096)) rejected { ConnectionPeer.parse("a".repeat(size)) }
        for (value in 0..255) {
            val input = "a".repeat(63) + value.toChar()
            val valid = value.toChar() in '0'..'9' || value.toChar() in 'a'..'f'
            assertEquals(valid, runCatching { ConnectionPeer.parse(input) }.isSuccess)
        }
        rejected { peer('a').copy(fingerprint = "invalid") }
    }
}
