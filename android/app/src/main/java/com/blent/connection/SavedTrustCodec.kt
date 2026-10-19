package com.blent.connection

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.DataOutputStream

/** Flat private metadata. Pages retain the existing <=4096-byte event bound. */
internal object SavedTrustCodec {
    const val REQUEST_LIMIT = 142
    fun request(value: SavedTrustRequest): ByteArray {
        val bytes = ByteArrayOutputStream()
        DataOutputStream(bytes).use { out ->
            out.writeByte(1)
            when (value) {
                SavedTrustRequest.List -> out.writeByte(0)
                is SavedTrustRequest.Forget -> {
                    out.writeByte(1)
                    ConnectionAuthorityCodec.peer(out, value.selected.local)
                    out.writeLong(value.selected.revision)
                    ConnectionAuthorityCodec.peer(out, value.selected.peer)
                }
            }
        }
        return bytes.toByteArray().also { require(it.size in 2..REQUEST_LIMIT) }
    }
    fun request(bytes: ByteArray): SavedTrustRequest {
        require(bytes.size in 2..REQUEST_LIMIT)
        val input = DataInputStream(ByteArrayInputStream(bytes))
        require(input.readUnsignedByte() == 1)
        val value = when (input.readUnsignedByte()) {
            0 -> SavedTrustRequest.List
            1 -> SavedTrustRequest.Forget(SavedComputerSelection(
                ConnectionAuthorityCodec.peer(input), input.readLong(), ConnectionAuthorityCodec.peer(input),
            ))
            else -> error("Unknown saved-trust request")
        }
        require(input.available() == 0)
        return value
    }
    fun page(out: DataOutputStream, value: SavedComputersPage) {
        out.writeBoolean(value.local != null)
        value.local?.let { ConnectionAuthorityCodec.peer(out, it) }
        out.writeLong(value.revision); out.writeInt(value.total); out.writeInt(value.offset)
        val peers = value.peers
        out.writeInt(peers.size)
        peers.forEach { ConnectionAuthorityCodec.peer(out, it) }
    }
    fun page(input: DataInputStream): SavedComputersPage {
        val local = if (ConnectionControlFields.bool(input)) ConnectionAuthorityCodec.peer(input) else null
        val revision = input.readLong(); val total = input.readInt(); val offset = input.readInt()
        val count = input.readInt()
        require(count in 0..SavedComputersPage.CAPACITY)
        return SavedComputersPage(local, revision, total, offset, List(count) { ConnectionAuthorityCodec.peer(input) })
    }
}
