package com.blent.connection

import com.blent.MediaLocalConsent
import java.io.DataInputStream
import java.io.DataOutputStream

/** Private audio control schema, independent of OS/player/capture implementations.
 * Fixed metadata only; no PCM, recursive payloads or remote-selected address. */
internal object ConnectionAudioCodec {
    private fun lease(out: DataOutputStream, value: ConnectionResourceLease) {
        ConnectionAudioLease.validate(value)
        out.writeByte(value.key.resource.wire)
        ConnectionAuthorityCodec.lease(out, value, value.key.resource)
    }
    private fun lease(input: DataInputStream): ConnectionResourceLease {
        val resource = when (input.readUnsignedByte()) {
            3 -> ConnectionManagement.Resource.MICROPHONE
            4 -> ConnectionManagement.Resource.SPEAKERS
            else -> error("Invalid audio resource")
        }
        return ConnectionAuthorityCodec.lease(input, resource)
    }
    private fun choice(out: DataOutputStream, value: MediaLocalConsent) {
        out.writeBoolean(value.foreground); out.writeBoolean(value.permission)
        out.writeBoolean(value.enabled); out.writeBoolean(value.backgroundAllowed)
    }
    private fun choice(input: DataInputStream): MediaLocalConsent = MediaLocalConsent(
        ConnectionControlFields.bool(input), ConnectionControlFields.bool(input),
        ConnectionControlFields.bool(input), ConnectionControlFields.bool(input),
    )
    private fun profile(out: DataOutputStream, key: ConnectionManagement.Key, value: ConnectionManagement.Audio) {
        val body = ManagementProfiles.encode(ConnectionManagement.Offer(key, ConnectionManagement.Credential(ByteArray(32)), value))
        out.write(body, 32, 4)
    }
    private fun profile(input: DataInputStream, key: ConnectionManagement.Key): ConnectionManagement.Audio {
        val body = ByteArray(36); input.readFully(body, 32, 4)
        return ManagementProfiles.decode(key, body).profile as ConnectionManagement.Audio
    }
    fun command(out: DataOutputStream, value: ConnectionWorkerCommand.Audio) {
        when (value) {
            is ConnectionWorkerCommand.AudioChoice -> { out.writeByte(6); lease(out, value.lease); choice(out, value.choice) }
            is ConnectionWorkerCommand.AudioConsumerRetired -> { out.writeByte(7); lease(out, value.lease) }
        }
    }
    fun command(input: DataInputStream, code: Int): ConnectionWorkerCommand.Audio = when (code) {
        6 -> ConnectionWorkerCommand.AudioChoice(lease(input), choice(input))
        7 -> ConnectionWorkerCommand.AudioConsumerRetired(lease(input))
        else -> error("Unknown audio command")
    }
    fun event(out: DataOutputStream, value: ConnectionWorkerEvent.Audio) {
        when (value) {
            is ConnectionWorkerEvent.AudioOffered -> { out.writeByte(11); offered(out, value.offer) }
            is ConnectionWorkerEvent.AudioRoute -> { out.writeByte(12); route(out, value.route) }
            is ConnectionWorkerEvent.AudioClosed -> { out.writeByte(13); lease(out, value.lease) }
        }
    }
    fun event(input: DataInputStream, code: Int): ConnectionWorkerEvent.Audio = when (code) {
        11 -> ConnectionWorkerEvent.AudioOffered(offered(input))
        12 -> ConnectionWorkerEvent.AudioRoute(route(input))
        13 -> ConnectionWorkerEvent.AudioClosed(lease(input))
        else -> error("Unknown audio event")
    }
    private fun offered(out: DataOutputStream, value: LocalAudioOffer) {
        lease(out, value.lease); profile(out, value.lease.key, value.profile)
    }
    private fun offered(input: DataInputStream): LocalAudioOffer {
        val lease = lease(input); return LocalAudioOffer(lease, profile(input, lease.key))
    }
    private fun route(out: DataOutputStream, value: LocalAudioRoute) {
        lease(out, value.lease); out.writeInt(value.port); out.write(value.credential.bytes()); profile(out, value.lease.key, value.profile)
    }
    private fun route(input: DataInputStream): LocalAudioRoute {
        val lease = lease(input); val port = input.readInt()
        val credential = ConnectionManagement.Credential(ByteArray(32).also(input::readFully))
        return LocalAudioRoute(lease, credential, port, profile(input, lease.key))
    }
}
