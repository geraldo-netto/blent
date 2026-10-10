package com.blent.connection

import java.io.DataInputStream
import java.io.DataOutputStream

/** Strict booleans shared by flat private metadata codecs. */
internal object ConnectionControlFields {
    fun bool(input: DataInputStream): Boolean = when (input.readUnsignedByte()) {
        0 -> false
        1 -> true
        else -> error("Invalid control boolean")
    }
}

/** Camera-only private schema. Profile validation is owned by management's
 * canonical contract; no OS handles, hardware types or encoded media here. */
internal object ConnectionCameraCodec {
    private val resource = ConnectionManagement.Resource.CAMERA
    private fun lease(out: DataOutputStream, value: ConnectionResourceLease) = ConnectionAuthorityCodec.lease(out, value, resource)
    private fun lease(input: DataInputStream): ConnectionResourceLease = ConnectionAuthorityCodec.lease(input, resource)
    private fun choice(out: DataOutputStream, value: CameraConsentChoice) {
        out.writeBoolean(value.foreground); out.writeBoolean(value.permission)
        out.writeBoolean(value.enabled); out.writeBoolean(value.backgroundAllowed)
    }
    private fun choice(input: DataInputStream): CameraConsentChoice = CameraConsentChoice(
        ConnectionControlFields.bool(input), ConnectionControlFields.bool(input),
        ConnectionControlFields.bool(input), ConnectionControlFields.bool(input),
    )
    private fun profile(out: DataOutputStream, key: ConnectionManagement.Key, value: ConnectionManagement.Camera) {
        val body = ManagementProfiles.encode(ConnectionManagement.Offer(key, ConnectionManagement.Credential(ByteArray(32)), value))
        out.write(body, 32, 13)
    }
    private fun profile(input: DataInputStream, key: ConnectionManagement.Key): ConnectionManagement.Camera {
        val body = ByteArray(45)
        input.readFully(body, 32, 13)
        return ManagementProfiles.decode(key, body).profile as ConnectionManagement.Camera
    }
    fun command(out: DataOutputStream, value: ConnectionWorkerCommand.Camera) {
        when (value) {
            is ConnectionWorkerCommand.CameraChoice -> { out.writeByte(3); lease(out, value.lease); choice(out, value.choice) }
            is ConnectionWorkerCommand.CameraPermission -> {
                out.writeByte(4); lease(out, value.lease); out.writeBoolean(value.granted); choice(out, value.choice)
            }
            is ConnectionWorkerCommand.CameraConsumerRetired -> { out.writeByte(5); lease(out, value.lease) }
        }
    }
    fun command(input: DataInputStream, code: Int): ConnectionWorkerCommand.Camera = when (code) {
        3 -> ConnectionWorkerCommand.CameraChoice(lease(input), choice(input))
        4 -> ConnectionWorkerCommand.CameraPermission(lease(input), ConnectionControlFields.bool(input), choice(input))
        5 -> ConnectionWorkerCommand.CameraConsumerRetired(lease(input))
        else -> error("Unknown camera command")
    }
    fun offered(out: DataOutputStream, value: LocalCameraOffer) {
        lease(out, value.lease); profile(out, value.lease.key, value.profile); out.writeBoolean(value.permissionRequired)
    }
    fun offered(input: DataInputStream): LocalCameraOffer {
        val lease = lease(input)
        return LocalCameraOffer(lease, profile(input, lease.key), ConnectionControlFields.bool(input))
    }
    fun route(out: DataOutputStream, value: LocalCameraRoute) {
        lease(out, value.lease); out.writeInt(value.port); out.write(value.credential.bytes())
        profile(out, value.lease.key, value.profile)
    }
    fun route(input: DataInputStream): LocalCameraRoute {
        val lease = lease(input); val port = input.readInt()
        val credential = ConnectionManagement.Credential(ByteArray(32).also(input::readFully))
        return LocalCameraRoute(lease, credential, port, profile(input, lease.key))
    }
    fun closed(out: DataOutputStream, value: ConnectionResourceLease) = lease(out, value)
    fun closed(input: DataInputStream): ConnectionResourceLease = lease(input)
}

/** Media metadata tags are disjoint and preserve the existing display schema. */
internal object ConnectionMediaCodec {
    fun write(out: DataOutputStream, value: ConnectionWorkerEvent.Media) {
        when (value) {
            is ConnectionWorkerEvent.Display -> { out.writeByte(6); ConnectionDisplayCodec.write(out, value.route) }
            is ConnectionWorkerEvent.DisplayClosed -> { out.writeByte(7); ConnectionAuthorityCodec.lease(out, value.lease) }
            is ConnectionWorkerEvent.CameraOffered -> { out.writeByte(8); ConnectionCameraCodec.offered(out, value.offer) }
            is ConnectionWorkerEvent.Camera -> { out.writeByte(9); ConnectionCameraCodec.route(out, value.route) }
            is ConnectionWorkerEvent.CameraClosed -> { out.writeByte(10); ConnectionCameraCodec.closed(out, value.lease) }
        }
    }
    fun read(input: DataInputStream, code: Int): ConnectionWorkerEvent.Media = when (code) {
        6 -> ConnectionWorkerEvent.Display(ConnectionDisplayCodec.read(input))
        7 -> ConnectionWorkerEvent.DisplayClosed(ConnectionAuthorityCodec.lease(input))
        8 -> ConnectionWorkerEvent.CameraOffered(ConnectionCameraCodec.offered(input))
        9 -> ConnectionWorkerEvent.Camera(ConnectionCameraCodec.route(input))
        10 -> ConnectionWorkerEvent.CameraClosed(ConnectionCameraCodec.closed(input))
        else -> error("Unknown media event")
    }
}
