package com.blent.connection

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.DataOutputStream
import java.util.Random
import org.junit.Assert.*
import org.junit.Test

internal fun cameraLease(owner: Long = 1): ConnectionResourceLease = ConnectionResourceLease(
    ConnectionTicket(1, ConnectionPeer.parse("ab".repeat(32))),
    ConnectionManagement.Key(ConnectionManagement.Resource.CAMERA, owner, owner), owner,
)
internal fun cameraProfile() = ConnectionManagement.Camera(640, 480, 30, 2000, 500, 500, 1, false, true)
internal fun cameraRoute(owner: Long = 1) = LocalCameraRoute(cameraLease(owner),
    ConnectionManagement.Credential(ByteArray(32) { it.toByte() }), 19899, cameraProfile())

internal fun assertWorkerMetadata(expected: ConnectionWorkerEvent, actual: ConnectionWorkerEvent) {
    assertArrayEquals(ConnectionWorkerCodec.event(expected), ConnectionWorkerCodec.event(actual))
}

class ConnectionCameraCodecTest {
    private fun rejected(action: () -> Unit) { assertThrows(Exception::class.java) { action() } }
    private fun commands(): List<ConnectionWorkerCommand.Camera> = (0..15).flatMap { flags ->
        val choice = CameraConsentChoice(flags and 1 != 0, flags and 2 != 0, flags and 4 != 0, flags and 8 != 0)
        listOf(ConnectionWorkerCommand.CameraChoice(cameraLease(), choice),
            ConnectionWorkerCommand.CameraPermission(cameraLease(), false, choice),
            ConnectionWorkerCommand.CameraPermission(cameraLease(), true, choice))
    } + ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())
    private fun events(): List<ConnectionWorkerEvent> = listOf(
        ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)),
        ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), true)),
        ConnectionWorkerEvent.Camera(cameraRoute()), ConnectionWorkerEvent.CameraClosed(cameraLease()),
        ConnectionWorkerEvent.Display(displayRoute()), ConnectionWorkerEvent.DisplayClosed(displayRoute().lease),
    )
    @Test fun t939CameraMetadataAndConsentRoundTripWithoutMedia() {
        for (command in commands()) {
            val bytes = ConnectionWorkerCodec.command(command)
            assertTrue(bytes.size <= 110); assertEquals(command, ConnectionWorkerCodec.command(bytes))
        }
        for (event in events()) {
            val bytes = ConnectionWorkerCodec.event(event)
            assertTrue(bytes.size <= 160); assertWorkerMetadata(event, ConnectionWorkerCodec.event(bytes))
        }
        assertEquals(6, ConnectionWorkerCodec.event(ConnectionWorkerEvent.Display(displayRoute()))[1].toInt())
        assertEquals(7, ConnectionWorkerCodec.event(ConnectionWorkerEvent.DisplayClosed(displayRoute().lease))[1].toInt())
    }
    @Test fun t939AllTruncationsTrailingVersionsAndUnknownTagsReject() {
        for (command in commands()) {
            val bytes = ConnectionWorkerCodec.command(command)
            for (size in 0 until bytes.size) rejected { ConnectionWorkerCodec.command(bytes.copyOf(size)) }
            rejected { ConnectionWorkerCodec.command(bytes + 0) }
            rejected { ConnectionWorkerCodec.command(bytes.copyOf().also { it[0] = 2 }) }
        }
        for (event in events()) {
            val bytes = ConnectionWorkerCodec.event(event)
            for (size in 0 until bytes.size) rejected { ConnectionWorkerCodec.event(bytes.copyOf(size)) }
            rejected { ConnectionWorkerCodec.event(bytes + 0) }
            rejected { ConnectionWorkerCodec.event(bytes.copyOf().also { it[0] = 2 }) }
        }
        for (tag in listOf(0, 11, 127, 255)) {
            rejected { ConnectionWorkerCodec.command(byteArrayOf(1, tag.toByte())) }
            rejected { ConnectionWorkerCodec.event(byteArrayOf(1, tag.toByte())) }
        }
    }
    @Test fun t939StrictBooleanLeasePortAndProfileBoundsReject() {
        val permission = ConnectionWorkerCodec.command(ConnectionWorkerCommand.CameraPermission(cameraLease(), true, CameraConsentChoice(true, true)))
        for (index in permission.size - 5 until permission.size) for (byte in listOf(2, 127, 255)) {
            rejected { ConnectionWorkerCodec.command(permission.copyOf().also { it[index] = byte.toByte() }) }
        }
        val offered = ConnectionWorkerCodec.event(events()[0])
        rejected { ConnectionWorkerCodec.event(offered.copyOf().also { it[it.lastIndex] = 2 }) }
        val route = ConnectionWorkerCodec.event(ConnectionWorkerEvent.Camera(cameraRoute()))
        // The profile occupies the final canonical 13 bytes. Flags and lens reject invalid values.
        for (index in listOf(route.lastIndex, route.lastIndex - 1)) {
            rejected { ConnectionWorkerCodec.event(route.copyOf().also { it[index] = 127 }) }
        }
        for (value in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE)) {
            rejected { cameraRoute().copy(port = value) }
        }
        for (value in listOf(Long.MIN_VALUE, -1, 0)) {
            val lease = cameraLease().copy(owner = value)
            rejected { ConnectionWorkerCommand.CameraChoice(lease, CameraConsentChoice(true, true)) }
            rejected { ConnectionWorkerCommand.CameraPermission(lease, true, CameraConsentChoice(true, true)) }
            rejected { ConnectionWorkerCommand.CameraConsumerRetired(lease) }
            rejected { ConnectionWorkerEvent.CameraClosed(lease) }
        }
        val display = displayRoute().lease
        rejected { ConnectionWorkerCommand.CameraChoice(display, CameraConsentChoice(true, true)) }
        rejected { ConnectionWorkerCommand.CameraPermission(display, true, CameraConsentChoice(true, true)) }
        rejected { ConnectionWorkerCommand.CameraConsumerRetired(display) }
        rejected { ConnectionWorkerEvent.CameraClosed(display) }
        for (value in listOf(Int.MIN_VALUE, -1, 0, 159, 161, 1922, Int.MAX_VALUE)) {
            rejected { LocalCameraOffer(cameraLease(), cameraProfile().copy(width = value), false) }
        }
    }
    @Test fun t939BoundaryProfilesUseCanonicalManagementContract() {
        for (profile in listOf(
            ConnectionManagement.Camera(160, 120, 5, 256, 256, 50, 0, false, false),
            ConnectionManagement.Camera(1920, 1080, 30, 20000, 20000, 2000, 1, true, true),
        )) for (port in listOf(1, 65535)) {
            val event = ConnectionWorkerEvent.Camera(cameraRoute().copy(profile = profile, port = port))
            assertWorkerMetadata(event, ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(event)))
        }
    }
    @Test fun t939SeededBoundedMutationRoundTripsOnlyValidatedRecords() {
        val random = Random(939)
        repeat(1024) {
            val original = commands()[random.nextInt(49)]
            val bytes = ConnectionWorkerCodec.command(original)
            bytes[random.nextInt(bytes.size)] = random.nextInt(256).toByte()
            val decoded = runCatching { ConnectionWorkerCodec.command(bytes) }.getOrNull()
            if (decoded != null) assertEquals(decoded, ConnectionWorkerCodec.command(ConnectionWorkerCodec.command(decoded)))
            val event = events()[random.nextInt(6)]
            val record = ConnectionWorkerCodec.event(event)
            record[random.nextInt(record.size)] = random.nextInt(256).toByte()
            val metadata = runCatching { ConnectionWorkerCodec.event(record) }.getOrNull()
            if (metadata != null) assertWorkerMetadata(metadata, ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(metadata)))
        }
    }
    @Test fun t939FocusedCodecsRejectForeignTagsAndLeaseTypes() {
        val input = { DataInputStream(ByteArrayInputStream(byteArrayOf())) }
        rejected { ConnectionCameraCodec.command(input(), 2) }
        rejected { ConnectionMediaCodec.read(input(), 5) }
        val out = DataOutputStream(ByteArrayOutputStream())
        rejected { ConnectionAuthorityCodec.lease(out, cameraLease()) }
        rejected { ConnectionCameraCodec.closed(out, displayRoute().lease) }
    }
    @Test fun t939CameraCommandsStayUnsupportedWithoutRuntimeComposition() {
        val rig = RuntimeRig(); rig.start()
        for (command in commands()) assertFalse(rig.owner.command(command))
        for (event in events()) rig.setup.events.add(event)
        assertNull(rig.owner.poll()); assertFalse(rig.run.stopped)
    }
}
