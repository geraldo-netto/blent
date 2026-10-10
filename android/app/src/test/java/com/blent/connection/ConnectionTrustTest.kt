package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.nio.ByteBuffer

@org.junit.runner.RunWith(org.robolectric.RobolectricTestRunner::class)
@org.robolectric.annotation.Config(sdk = [27, 34])
class ConnectionTrustTest {
    private fun peer(value: Char) = ConnectionPeer.parse(value.toString().repeat(64))
    private fun rejected(body: () -> Unit) { assertThrows(Exception::class.java, body) }
    private class Store : PeerTrustStore {
        var bytes: ByteArray? = null
        var failure = false
        var closed = 0
        override fun load(): ByteArray? { check(!failure); return bytes }
        override fun replace(record: ByteArray) { check(!failure); bytes = record.copyOf() }
        override fun close() { closed++; check(!failure) }
    }
    private fun lease(pairing: ConnectionPairing, peer: ConnectionPeer, remember: Boolean = true): ConnectionApproval {
        val ticket = pairing.begin(peer, false, 0, 100)
        pairing.approve(ticket, remember, 1)
        return pairing.authenticated(ticket, peer, 2)
    }

    @Test fun t868SharedTrustVectorsMatchExactWire() {
        val text = javaClass.classLoader!!.getResourceAsStream("connection-trust-v1.json")!!
            .bufferedReader().use { it.readText() }
        val fixture = org.json.JSONObject(text)
        val valid = fixture.getJSONArray("valid")
        for (index in 0 until valid.length()) {
            val item = valid.getJSONObject(index)
            val bytes = hex(item.getString("hex"))
            val parsed = ConnectionTrustCodec.decode(bytes)
            assertEquals(item.getString("local"), parsed.local.fingerprint)
            assertEquals(item.getString("revision").toLong(), parsed.revision)
            val peers = item.getJSONArray("peers")
            assertEquals(List(peers.length()) { peers.getString(it) }, parsed.peers.map { it.fingerprint })
            assertArrayEquals(bytes, ConnectionTrustCodec.encode(parsed))
        }
        val invalid = fixture.getJSONArray("invalid")
        for (index in 0 until invalid.length()) rejected {
            ConnectionTrustCodec.decode(hex(invalid.getJSONObject(index).getString("hex")))
        }
    }
    private fun hex(value: String) = value.chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    @Test fun t868CanonicalCodecBoundariesAndBoundedMutations() {
        val mutable = mutableListOf(peer('b'), peer('c'))
        val snapshot = TrustSnapshot(peer('a'), Long.MAX_VALUE, mutable)
        mutable.clear()
        assertEquals(2, snapshot.peers.size)
        val bytes = ConnectionTrustCodec.encode(snapshot)
        val restored = ConnectionTrustCodec.decode(bytes)
        assertEquals(snapshot.local, restored.local)
        assertEquals(Long.MAX_VALUE, restored.revision)
        assertEquals(snapshot.peers, restored.peers)
        for (size in 0 until bytes.size) rejected { ConnectionTrustCodec.decode(bytes.copyOf(size)) }
        rejected { ConnectionTrustCodec.decode(bytes + byteArrayOf(0)) }
        rejected { ConnectionTrustCodec.decode(ByteArray(ConnectionTrustCodec.MAX_RECORD + 1)) }
        rejected { TrustSnapshot(peer('a'), 0, emptyList()) }
        rejected { TrustSnapshot(peer('a'), -1, emptyList()) }
        rejected { TrustSnapshot(peer('a'), 1, List(1025) { peer('b') }) }
        rejected { TrustSnapshot(peer('a'), 1, listOf(peer('b'), peer('b'))) }
        rejected { TrustSnapshot(peer('a'), 1, listOf(peer('c'), peer('b'))) }
        repeat(4096) { index ->
            val bad = bytes.copyOf()
            ByteBuffer.wrap(bad).putInt(80, index + 1025)
            rejected { ConnectionTrustCodec.decode(bad) }
        }
        for (index in listOf(0, 7, 8, 71, 84, 87, 88)) {
            val bad = bytes.copyOf(); bad[index] = (-1).toByte()
            rejected { ConnectionTrustCodec.decode(bad) }
        }
        val maximum = (0 until 1024).map { ConnectionPeer.parse(it.toString(16).padStart(64, '0')) }
        val full = ConnectionTrustCodec.encode(TrustSnapshot(peer('a'), 1, maximum))
        assertEquals(ConnectionTrustCodec.MAX_RECORD, full.size)
        assertEquals(maximum, ConnectionTrustCodec.decode(full).peers)
    }

    @Test fun t868RememberRequiresCurrentProofAndRestartReusesOnlyMatchingIdentity() {
        val store = Store(); val pairing = ConnectionPairing(); val trust = ConnectionTrust(store, peer('a'))
        assertFalse(trust.trusted(peer('b')))
        val ticket = pairing.begin(peer('b'), false, 0, 100)
        rejected { trust.remember(pairing, ConnectionApproval(ticket, true)) }
        pairing.approve(ticket, true, 1)
        rejected { trust.remember(pairing, ConnectionApproval(ticket, true)) }
        val lease = pairing.authenticated(ticket, peer('b'), 2)
        assertTrue(trust.remember(pairing, lease)); assertFalse(trust.remember(pairing, lease))
        assertTrue(trust.trusted(peer('b')))
        rejected { trust.remember(pairing, lease.copy(remember = false)) }
        trust.close(); trust.close(); assertEquals(1, store.closed)
        assertFalse(trust.trusted(peer('b')))
        rejected { trust.remember(pairing, lease) }
        val restarted = ConnectionTrust(store, peer('a')); assertTrue(restarted.trusted(peer('b')))
        assertTrue(restarted.revoke(pairing, peer('b')))
        assertEquals(ConnectionPairing.Phase.CLOSING, pairing.phase)
        rejected { restarted.remember(pairing, lease) }
        assertFalse(restarted.revoke(pairing, peer('b')))
        assertEquals(2L, ConnectionTrustCodec.decode(store.bytes!!).revision)
        assertFalse(ConnectionTrust(store, peer('a')).trusted(peer('b')))
        rejected { ConnectionTrust(store, peer('c')) }
    }

    @Test fun t868FailuresPoisonAdmissionAndRevokeCannotRestoreLiveAuthority() {
        val store = Store(); val pairing = ConnectionPairing(); val trust = ConnectionTrust(store, peer('a'))
        val lease = lease(pairing, peer('b'))
        trust.remember(pairing, lease)
        store.failure = true
        rejected { trust.revoke(pairing, peer('b')) }
        assertFalse(trust.trusted(peer('b'))); assertFalse(pairing.current(lease))
        store.failure = false
        rejected { trust.remember(pairing, lease) }
        rejected { trust.revoke(pairing, peer('b')) }
        val fresh = ConnectionPairing(); val temporary = lease(fresh, peer('c'), false)
        val other = ConnectionTrust(Store(), peer('a'))
        rejected { other.remember(fresh, temporary) }
        assertFalse(other.revoke(fresh, peer('c')))
        assertFalse(fresh.current(temporary))
        val broken = Store().apply { failure = true }
        val error = assertThrows(Exception::class.java) { ConnectionTrust(broken, peer('a')) }
        assertEquals(1, broken.closed); assertEquals(1, error.suppressed.size)
        val malformed = Store().apply { bytes = byteArrayOf(1) }
        rejected { ConnectionTrust(malformed, peer('a')) }; assertEquals(1, malformed.closed)
    }

    @Test fun t868CapacityAndRevisionCannotEvictOrWrap() {
        for (capacity in listOf(-1, 0, 1025)) rejected { ConnectionTrust(Store(), peer('a'), capacity) }
        val store = Store(); val first = ConnectionPairing(); val second = ConnectionPairing()
        val trust = ConnectionTrust(store, peer('a'), 1)
        trust.remember(first, lease(first, peer('b')))
        rejected { trust.remember(second, lease(second, peer('c'))) }
        assertTrue(trust.trusted(peer('b'))); assertFalse(trust.trusted(peer('c')))
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(peer('a'), 1, listOf(peer('b'), peer('c'))))
        rejected { ConnectionTrust(store, peer('a'), 1) }
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(peer('a'), Long.MAX_VALUE, listOf(peer('b'))))
        val exhausted = ConnectionTrust(store, peer('a'))
        rejected { exhausted.revoke(first, peer('b')) }
        assertFalse(exhausted.trusted(peer('b')))
        assertEquals(Long.MAX_VALUE, ConnectionTrustCodec.decode(store.bytes!!).revision)
        val failure = Store().apply { failure = true }
        // Closing a successfully constructed owner remains fail-closed even if native close fails.
        failure.failure = false
        val closing = ConnectionTrust(failure, peer('a')); failure.failure = true
        rejected { closing.close() }; closing.close()
        assertFalse(closing.trusted(peer('a')))
    }
}
