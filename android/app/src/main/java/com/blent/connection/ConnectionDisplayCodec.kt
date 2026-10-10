package com.blent.connection

import java.io.DataInputStream
import java.io.DataOutputStream

/** Shared identity/lease fields for the private bounded worker codec. Parsed
 * values are metadata, never proof that pairing or native resources are live. */
internal object ConnectionAuthorityCodec {
    fun peer(output: DataOutputStream, value: ConnectionPeer) = output.writeUTF(value.fingerprint)
    fun peer(input: DataInputStream): ConnectionPeer = ConnectionPeer.parse(input.readUTF())
    fun ticket(output: DataOutputStream, value: ConnectionTicket) {
        require(value.epoch > 0); output.writeLong(value.epoch); peer(output, value.peer)
    }
    fun ticket(input: DataInputStream): ConnectionTicket {
        val epoch = input.readLong(); require(epoch > 0)
        return ConnectionTicket(epoch, peer(input))
    }
    fun lease(output: DataOutputStream, value: ConnectionResourceLease, resource: ConnectionManagement.Resource = ConnectionManagement.Resource.DISPLAY) {
        ConnectionLocalLease.validate(value, resource)
        ticket(output, value.connection)
        output.writeLong(value.owner); output.writeLong(value.key.request); output.writeLong(value.key.generation)
    }
    fun lease(input: DataInputStream, resource: ConnectionManagement.Resource = ConnectionManagement.Resource.DISPLAY): ConnectionResourceLease {
        val connection = ticket(input)
        val owner = input.readLong()
        val key = ConnectionManagement.Key(resource, input.readLong(), input.readLong())
        return ConnectionResourceLease(connection, key, owner).also { ConnectionLocalLease.validate(it, resource) }
    }
}

/** Fixed-size route payload nested inside the existing <=4096 byte IPC bound.
 * Credentials remain private same-UID metadata and are never formatted in logs. */
internal object ConnectionDisplayCodec {
    fun write(output: DataOutputStream, value: LocalDisplayRoute) {
        ConnectionAuthorityCodec.lease(output, value.lease)
        output.writeInt(value.controlPort); output.writeInt(value.displayPort)
        output.write(value.credential.bytes())
    }
    fun read(input: DataInputStream): LocalDisplayRoute {
        val lease = ConnectionAuthorityCodec.lease(input)
        val control = input.readInt(); val display = input.readInt()
        val credential = ByteArray(32); input.readFully(credential)
        return LocalDisplayRoute(lease, ConnectionManagement.Credential(credential), control, display)
    }
}
