package com.blent.connection

import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import android.os.Process
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowBinder

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionWorkerIpcTest {
    private class Endpoint : Binder() {
        var calls = 0
        var reject = false
        var event: ConnectionWorkerEvent? = null
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            ConnectionWorkerIpc.validate(data, if (event == null) ConnectionWorkerIpc.REQUEST else ConnectionWorkerIpc.EVENT, flags)
            assertEquals(7L, ConnectionWorkerIpc.generation(data))
            if (event != null) event = ConnectionWorkerCodec.event(ConnectionWorkerIpc.bytes(data))
            else when (code) {
                ConnectionWorkerIpc.START -> {
                    assertEquals(ConnectionWorkerSpec("usb"), ConnectionWorkerCodec.spec(ConnectionWorkerIpc.bytes(data)))
                    assertNotNull(data.readStrongBinder())
                }
                ConnectionWorkerIpc.CONTROL -> assertTrue(ConnectionWorkerCodec.command(ConnectionWorkerIpc.bytes(data)) is ConnectionWorkerCommand.Revoke)
            }
            ConnectionWorkerIpc.end(data); calls += 1
            return !reject
        }
    }
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    @Test fun t884OneWayPrivateRequestsAndEventsAreExactlyBounded() {
        val endpoint = Endpoint()
        ConnectionWorkerIpc.command(endpoint, 1, 7, ConnectionWorkerCodec.spec(ConnectionWorkerSpec("usb")), Binder())
        val command = ConnectionWorkerCommand.Revoke(ConnectionPeer.parse("ab".repeat(32)))
        ConnectionWorkerIpc.command(endpoint, 2, 7, ConnectionWorkerCodec.command(command))
        ConnectionWorkerIpc.command(endpoint, 3, 7); ConnectionWorkerIpc.command(endpoint, 4, 7)
        assertEquals(4, endpoint.calls)
        val event = ConnectionWorkerEvent.Ended(null); endpoint.event = event
        ConnectionWorkerIpc.event(endpoint, 7, event); assertEquals(event, endpoint.event)
        endpoint.reject = true; rejected { ConnectionWorkerIpc.event(endpoint, 7, event) }
    }
    @Test fun t884InvalidSendParametersNeverReachRemote() {
        val endpoint = Endpoint()
        for (generation in listOf(Long.MIN_VALUE, -1, 0)) {
            rejected { ConnectionWorkerIpc.command(endpoint, 3, generation) }
            rejected { ConnectionWorkerIpc.event(endpoint, generation, ConnectionWorkerEvent.Ended(null)) }
        }
        for (code in listOf(Int.MIN_VALUE, 0, 5, Int.MAX_VALUE)) rejected { ConnectionWorkerIpc.command(endpoint, code, 7) }
        for (size in listOf(0, 1, ConnectionWorkerCodec.LIMIT + 1)) rejected { ConnectionWorkerIpc.command(endpoint, 2, 7, ByteArray(size)) }
        rejected { ConnectionWorkerIpc.command(endpoint, 1, 7) }
        rejected { ConnectionWorkerIpc.command(endpoint, 1, 7, byteArrayOf(1, 2)) }
        assertEquals(0, endpoint.calls)
    }
    private fun parcel(work: (Parcel) -> Unit) {
        val data = Parcel.obtain()
        try { work(data) } finally { data.recycle() }
    }
    @Test fun t884ForeignWrongDescriptorFlagsAndOversizeRejectAtBoundary() {
        parcel { data ->
            data.writeInterfaceToken(ConnectionWorkerIpc.REQUEST); data.setDataPosition(0)
            ShadowBinder.setCallingUid(Process.myUid() + 1)
            try { rejected { ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, 1) } }
            finally { ShadowBinder.setCallingUid(Process.myUid()) }
            rejected { ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, 0) }
            rejected { ConnectionWorkerIpc.validate(data, "wrong", 1) }
        }
        parcel { data -> rejected { ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, 1) } }
        parcel { data ->
            data.writeByteArray(ByteArray(ConnectionWorkerIpc.MAX_PARCEL + 1)); data.setDataPosition(0)
            rejected { ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, 1) }
        }
    }
    @Test fun t884NullOversizeAndTrailingPayloadsReject() {
        for (size in listOf(-1, 0, 1, ConnectionWorkerCodec.LIMIT + 1)) parcel { data ->
            data.writeByteArray(if (size < 0) null else ByteArray(size)); data.setDataPosition(0)
            rejected { ConnectionWorkerIpc.bytes(data) }
        }
        parcel { data -> data.writeLong(0); data.setDataPosition(0); rejected { ConnectionWorkerIpc.generation(data) } }
        parcel { data -> data.writeInt(1); data.setDataPosition(0); rejected { ConnectionWorkerIpc.end(data) } }
    }
    @Test fun t1227NativeBinderTransportFlagsPreserveOneWayValidation() {
        parcel { data ->
            data.writeInterfaceToken(ConnectionWorkerIpc.REQUEST)
            data.setDataPosition(0)
            ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, 17)
        }
    }

    @Test fun t1227OneWayBitIsIndependentOfOtherNativeFlagMetadata() {
        val values = (0..255).toList() + listOf(Int.MIN_VALUE, Int.MIN_VALUE + 1, Int.MAX_VALUE - 1, Int.MAX_VALUE)
        for (flags in values) parcel { data ->
            data.writeInterfaceToken(ConnectionWorkerIpc.REQUEST)
            data.setDataPosition(0)
            if (flags % 2 != 0) ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, flags)
            else rejected { ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, flags) }
        }
    }

}
