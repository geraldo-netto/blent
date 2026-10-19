package com.blent.connection

import java.nio.ByteBuffer
import java.nio.charset.StandardCharsets.US_ASCII

/** Canonical immutable trust data; pins are public certificate fingerprints. */
internal class TrustSnapshot(val local: ConnectionPeer, val revision: Long, peers: List<ConnectionPeer>) {
    private val values = boundedPeerCopy(peers, 1024)
    val peers: List<ConnectionPeer> get() = values.toList()
    init {
        require(revision > 0 && values.size <= 1024)
        require(values.zipWithNext().all { (left, right) -> left.fingerprint < right.fingerprint })
    }
}

internal object ConnectionTrustCodec {
    const val HEADER = 88
    const val MAX_RECORD = HEADER + 1024 * 64
    private val magic = byteArrayOf(66, 76, 84, 82, 0, 0, 0, 1)

    fun encode(snapshot: TrustSnapshot): ByteArray {
        val peers = snapshot.peers
        return ByteBuffer.allocate(HEADER + peers.size * 64).apply {
            put(magic)
            put(snapshot.local.fingerprint.toByteArray(US_ASCII))
            putLong(snapshot.revision)
            putInt(peers.size)
            putInt(0)
            peers.forEach { put(it.fingerprint.toByteArray(US_ASCII)) }
        }.array()
    }

    fun decode(bytes: ByteArray): TrustSnapshot {
        require(bytes.size in HEADER..MAX_RECORD)
        val buffer = ByteBuffer.wrap(bytes)
        require(ByteArray(8).also(buffer::get).contentEquals(magic))
        val local = pin(buffer)
        val revision = buffer.long
        val count = buffer.int
        require(count in 0..1024 && buffer.int == 0)
        require(bytes.size == HEADER + count * 64)
        return TrustSnapshot(local, revision, List(count) { pin(buffer) })
    }

    private fun pin(buffer: ByteBuffer): ConnectionPeer {
        val bytes = ByteArray(64).also(buffer::get)
        return ConnectionPeer.parse(String(bytes, US_ASCII))
    }
}
