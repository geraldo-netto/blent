package com.blent.connection

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.DataOutputStream

/** Flat private control schema. Strict types/bounds, no recursion or media bytes. */
internal object ConnectionWorkerCodec {
    const val LIMIT = 4096
    private fun encode(version: Int = 1, write: (DataOutputStream) -> Unit): ByteArray {
        val bytes = ByteArrayOutputStream()
        DataOutputStream(bytes).use { it.writeByte(version); write(it) }
        return bytes.toByteArray().also { require(it.size <= LIMIT) }
    }
    private fun <T> decode(bytes: ByteArray, version: Int = 1, read: (DataInputStream) -> T): T {
        require(bytes.size in 2..LIMIT)
        val input = DataInputStream(ByteArrayInputStream(bytes))
        require(input.readUnsignedByte() == version)
        return read(input).also { require(input.available() == 0) }
    }
    private fun string(output: DataOutputStream, value: String, limit: Int) {
        require(value.length <= limit && '\u0000' !in value)
        output.writeUTF(value)
    }
    private fun string(input: DataInputStream, limit: Int): String = input.readUTF().also {
        require(it.length <= limit && '\u0000' !in it)
    }
    private fun bool(input: DataInputStream): Boolean = when (input.readUnsignedByte()) {
        0 -> false
        1 -> true
        else -> error("Invalid control boolean")
    }
    fun spec(value: ConnectionWorkerSpec): ByteArray = encode(2) { out ->
        string(out, value.device, 512)
        out.writeInt(value.session.capacity); out.writeInt(value.session.messages); out.writeInt(value.session.quantum)
        out.writeLong(value.session.resourceTimeoutMs); out.writeLong(value.io.ioTimeoutMs)
        out.writeLong(value.io.retirementTimeoutMs); out.writeLong(value.approvalTimeoutMs)
        out.writeLong(value.nativeTimeoutMs); out.writeInt(value.trustedPeers)
        out.writeInt(value.accessoryCapacity); out.writeLong(value.timerMs)
    }
    fun spec(bytes: ByteArray): ConnectionWorkerSpec = decode(bytes, 2) { input ->
        val device = string(input, 512)
        val session = ConnectionSessionLimits(input.readInt(), input.readInt(), input.readInt(), input.readLong())
        ConnectionWorkerSpec(device, session, ConnectionRunLimits(input.readLong(), input.readLong()), input.readLong(),
            input.readLong(), input.readInt(), input.readInt(), input.readLong())
    }
    fun command(value: ConnectionWorkerCommand): ByteArray = encode { out ->
        when (value) {
            is ConnectionWorkerCommand.Decide -> {
                out.writeByte(1); ConnectionAuthorityCodec.ticket(out, value.ticket); out.writeBoolean(value.approved); out.writeBoolean(value.remember)
            }
            is ConnectionWorkerCommand.Revoke -> { out.writeByte(2); ConnectionAuthorityCodec.peer(out, value.peer) }
        }
    }
    fun command(bytes: ByteArray): ConnectionWorkerCommand = decode(bytes) { input ->
        when (input.readUnsignedByte()) {
            1 -> ConnectionWorkerCommand.Decide(ConnectionAuthorityCodec.ticket(input), bool(input), bool(input))
            2 -> ConnectionWorkerCommand.Revoke(ConnectionAuthorityCodec.peer(input))
            else -> error("Unknown connection command")
        }
    }
    fun event(value: ConnectionWorkerEvent): ByteArray = encode { out ->
        when (value) {
            is ConnectionWorkerEvent.Peer -> {
                out.writeByte(5); ConnectionAuthorityCodec.ticket(out, value.ticket); ConnectionAuthorityCodec.peer(out, value.local); out.writeBoolean(value.remembered)
            }
            is ConnectionWorkerEvent.Pairing -> { out.writeByte(1); ConnectionAuthorityCodec.ticket(out, value.ticket); ConnectionAuthorityCodec.peer(out, value.local) }
            is ConnectionWorkerEvent.State -> { out.writeByte(2); out.writeByte(value.phase.ordinal) }
            is ConnectionWorkerEvent.Trust -> { out.writeByte(3); ConnectionAuthorityCodec.peer(out, value.peer); out.writeBoolean(value.remembered) }
            is ConnectionWorkerEvent.Display -> { out.writeByte(6); ConnectionDisplayCodec.write(out, value.route) }
            is ConnectionWorkerEvent.DisplayClosed -> { out.writeByte(7); ConnectionAuthorityCodec.lease(out, value.lease) }
            is ConnectionWorkerEvent.Ended -> {
                out.writeByte(4); out.writeBoolean(value.failure != null)
                value.failure?.let { string(out, it, 512) }
            }
        }
    }
    fun event(bytes: ByteArray): ConnectionWorkerEvent = decode(bytes) { input ->
        when (input.readUnsignedByte()) {
            6 -> ConnectionWorkerEvent.Display(ConnectionDisplayCodec.read(input))
            7 -> ConnectionWorkerEvent.DisplayClosed(ConnectionAuthorityCodec.lease(input))
            5 -> ConnectionWorkerEvent.Peer(ConnectionAuthorityCodec.ticket(input), ConnectionAuthorityCodec.peer(input), bool(input))
            1 -> ConnectionWorkerEvent.Pairing(ConnectionAuthorityCodec.ticket(input), ConnectionAuthorityCodec.peer(input))
            2 -> {
                val code = input.readUnsignedByte(); require(code in ConnectionWorkerPhase.entries.indices)
                ConnectionWorkerEvent.State(ConnectionWorkerPhase.entries[code])
            }
            3 -> ConnectionWorkerEvent.Trust(ConnectionAuthorityCodec.peer(input), bool(input))
            4 -> ended(input)
            else -> error("Unknown connection event")
        }
    }
    private fun ended(input: DataInputStream) = ConnectionWorkerEvent.Ended(if (bool(input)) string(input, 512) else null)

}
