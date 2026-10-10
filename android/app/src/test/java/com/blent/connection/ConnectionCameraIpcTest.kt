package com.blent.connection

import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionCameraIpcTest {
    private class Endpoint(private val events: Boolean) : Binder() {
        var generation = 0L
        var value: Any? = null
        var rejected = false
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            ConnectionWorkerIpc.validate(data, if (events) ConnectionWorkerIpc.EVENT else ConnectionWorkerIpc.REQUEST, flags)
            generation = ConnectionWorkerIpc.generation(data)
            val bytes = ConnectionWorkerIpc.bytes(data)
            value = try { if (events) ConnectionWorkerCodec.event(bytes) else ConnectionWorkerCodec.command(bytes) }
            catch (_: Exception) { rejected = true; return false }
            ConnectionWorkerIpc.end(data)
            assertEquals(if (events) 1 else ConnectionWorkerIpc.CONTROL, code)
            assertEquals(IBinder.FLAG_ONEWAY, flags)
            return true
        }
    }
    @Test fun t939RealPrivateBinderCarriesCameraMetadataAndCommandsOnly() {
        val choices = CameraConsentChoice(true, false, true, false)
        val command = Endpoint(false)
        for (value in listOf(ConnectionWorkerCommand.CameraChoice(cameraLease(), choices),
            ConnectionWorkerCommand.CameraPermission(cameraLease(), false, choices),
            ConnectionWorkerCommand.CameraConsumerRetired(cameraLease()))) {
            ConnectionWorkerIpc.command(command, ConnectionWorkerIpc.CONTROL, 19, ConnectionWorkerCodec.command(value))
            assertEquals(value, command.value); assertEquals(19L, command.generation)
        }
        val event = Endpoint(true)
        for (value in listOf(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), true)),
            ConnectionWorkerEvent.Camera(cameraRoute()), ConnectionWorkerEvent.CameraClosed(cameraLease()))) {
            ConnectionWorkerIpc.event(event, 20, value)
            assertWorkerMetadata(value, event.value as ConnectionWorkerEvent); assertEquals(20L, event.generation)
        }
    }
    @Test fun t939BinderRejectsInvalidCameraBooleanBeforeDelivery() {
        val endpoint = Endpoint(false)
        val bytes = ConnectionWorkerCodec.command(ConnectionWorkerCommand.CameraPermission(cameraLease(), true, CameraConsentChoice(true, true)))
        bytes[bytes.lastIndex] = 2
        runCatching { ConnectionWorkerIpc.command(endpoint, ConnectionWorkerIpc.CONTROL, 1, bytes) }
        assertTrue(endpoint.rejected); assertNull(endpoint.value)
    }
}
