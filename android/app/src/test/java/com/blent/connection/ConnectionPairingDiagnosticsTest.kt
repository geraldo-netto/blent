package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionPairingDiagnosticsTest {
    @Test fun t1520DeniedPairingReportsReasonAndRetiresOriginalSetup() {
        for (remember in listOf(false, true)) SetupRig().use { rig ->
            assertTrue(rig.owner.decide(rig.ticket(), false, remember))
            setupAwait { rig.owner.retired() }
            assertEquals("Pairing denied on tablet", rig.owner.failure()?.message)
            assertEquals(1L, rig.backend.entered.count)
            assertFalse(rig.backend.ioMade)
            assertTrue(rig.backend.closed)
            assertTrue(rig.store.closed)
            assertNull(rig.store.bytes)
        }
    }

    @Test fun t1520FullTrustReportsCapacityWithoutEvictionOrLiveApproval() {
        for (capacity in listOf(1, 64, 1024)) {
            val local = ConnectionPeer.parse("a".repeat(64))
            val remote = ConnectionPeer.parse("b".repeat(64))
            val store = fullStore(local, capacity)
            val original = store.bytes!!.copyOf()
            ConnectionTrust(store, local, capacity).use { trust ->
                val pairing = ConnectionPairing()
                val ticket = trust.begin(pairing, remote, 0, 100)
                pairing.approve(ticket, true, 1)
                val failure = assertThrows(IllegalStateException::class.java) {
                    trust.authenticated(pairing, ticket, remote, 2)
                }
                assertEquals(capacityMessage(capacity), failure.message)
                assertEquals(ConnectionPairing.Phase.CLOSING, pairing.phase)
                assertFalse(trust.trusted(remote))
                assertEquals(capacity, trust.snapshot().peers.size)
                assertArrayEquals(original, store.bytes)
            }
            assertTrue(store.closed)
        }
    }

    @Test fun t1520FullSavedListReportsOwnerFailureAndAllowsTemporaryApproval() {
        for (remember in listOf(true, false)) {
            val backend = SetupBackend()
            val store = fullStore(setupPeer(backend.local), 64)
            val original = store.bytes!!.copyOf()
            SetupRig(backend, store).use { rig ->
                rig.owner.decide(rig.ticket(), true, remember)
                if (remember) {
                    setupAwait { rig.owner.retired() }
                    assertEquals(capacityMessage(64), rig.owner.failure()?.message)
                    assertFalse(backend.ioMade)
                } else {
                    assertTrue(rig.owner.current(rig.ready().approval))
                    assertNull(rig.owner.failure())
                }
                assertArrayEquals(original, store.bytes)
            }
            assertTrue(backend.closed)
            assertTrue(store.closed)
        }
    }

    private fun capacityMessage(capacity: Int) =
        "Saved computer limit reached ($capacity); forget a computer or pair without remembering"

    private fun fullStore(local: ConnectionPeer, capacity: Int): SetupStore {
        val peers = (0 until capacity).map { ConnectionPeer.parse(it.toString(16).padStart(64, '0')) }
        return SetupStore().apply { bytes = ConnectionTrustCodec.encode(TrustSnapshot(local, 1, peers)) }
    }
}
