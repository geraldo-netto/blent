package com.blent.connection

import android.content.Intent
import android.os.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger

class SavedTrustProbeService : ConnectionWorkerService() {
    internal var trust: (SavedTrustRequest, (ConnectionWorkerEvent) -> Unit) -> ConnectionWorker = { _, _ -> error("not configured") }
    internal var connect: (ConnectionWorkerSpec, (ConnectionWorkerEvent) -> Unit) -> ConnectionWorker = { _, _ -> error("USB factory forbidden") }
    val exits = AtomicInteger()
    override fun createTrustWorker(request: SavedTrustRequest, report: (ConnectionWorkerEvent) -> Unit) = trust(request, report)
    override fun createWorker(spec: ConnectionWorkerSpec, report: (ConnectionWorkerEvent) -> Unit) = connect(spec, report)
    override fun terminate() { exits.incrementAndGet() }
}
class SavedTrustDefaultService : ConnectionWorkerService() {
    override fun terminate() { /* Hosted process: no real application process exists. */ }
}
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class SavedTrustServiceTest {
    private class Events : Binder() {
        val values = CopyOnWriteArrayList<ConnectionWorkerEvent>()
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.EVENT, flags)
            assertEquals(9L, ConnectionWorkerIpc.generation(data))
            values.add(ConnectionWorkerCodec.event(ConnectionWorkerIpc.bytes(data)))
            ConnectionWorkerIpc.end(data); return true
        }
    }
    private class Held : ConnectionWorker {
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val cancels = AtomicInteger()
        override suspend fun execute() { entered.complete(Unit); release.await() }
        override fun command(value: ConnectionWorkerCommand) = false
        override fun cancel() { cancels.incrementAndGet() }
        override suspend fun retire() { release.await() }
    }
    private fun bind(service: ConnectionWorkerService) = service.onBind(Intent().putExtra("connection_generation", 9L))
    private fun trust(binder: IBinder, events: Events) = ConnectionWorkerIpc.command(binder,
        ConnectionWorkerIpc.START_TRUST, 9, SavedTrustCodec.request(SavedTrustRequest.List), events)
    private fun connect(binder: IBinder, events: Events) = ConnectionWorkerIpc.command(binder,
        ConnectionWorkerIpc.START, 9, ConnectionWorkerCodec.spec(ConnectionWorkerSpec("fixture")), events)
    private suspend fun until(condition: () -> Boolean) = withTimeout(5000) { while (!condition()) delay(1) }

    @Test fun t1151MaintenanceAndConnectionCannotReplaceAnAdmittedOriginalOwner() = runBlocking<Unit> {
        for (maintenance in listOf(true, false)) {
            val controller = Robolectric.buildService(SavedTrustProbeService::class.java).create()
            val service = controller.get(); val held = Held(); val events = Events()
            val madeTrust = AtomicInteger(); val madeConnection = AtomicInteger()
            service.trust = { _, _ -> madeTrust.incrementAndGet(); held }
            service.connect = { _, _ -> madeConnection.incrementAndGet(); held }
            try {
                val binder = bind(service)
                if (maintenance) trust(binder, events) else connect(binder, events)
                withTimeout(5000) { held.entered.await() }
                assertThrows(IllegalStateException::class.java) { trust(binder, events) }
                if (maintenance) assertThrows(IllegalStateException::class.java) { connect(binder, events) }
                assertEquals(0, service.exits.get()); assertEquals(0, held.cancels.get())
                assertEquals(if (maintenance) 1 else 0, madeTrust.get())
                assertEquals(if (maintenance) 0 else 1, madeConnection.get())
                ConnectionWorkerIpc.command(binder, ConnectionWorkerIpc.STOP, 9)
                until { held.cancels.get() == 1 }
                assertTrue(events.values.none { it is ConnectionWorkerEvent.Ended })
                held.release.complete(Unit)
                until { events.values.any { it is ConnectionWorkerEvent.Ended } }
            } finally { held.release.complete(Unit); controller.destroy() }
        }
    }
    @Test fun t1151StaleStoppedAndMalformedMaintenanceNeverCallNativeFactory() = runBlocking<Unit> {
        val controller = Robolectric.buildService(SavedTrustProbeService::class.java).create()
        val service = controller.get(); val calls = AtomicInteger(); val events = Events()
        service.trust = { _, _ -> calls.incrementAndGet(); Held() }
        try {
            val binder = bind(service)
            assertThrows(IllegalStateException::class.java) {
                ConnectionWorkerIpc.command(binder, ConnectionWorkerIpc.START_TRUST, 8,
                    SavedTrustCodec.request(SavedTrustRequest.List), events)
            }
            assertEquals(0, service.exits.get())
            ConnectionWorkerIpc.command(binder, ConnectionWorkerIpc.STOP, 9)
            assertThrows(IllegalStateException::class.java) { trust(binder, events) }
            assertEquals(0, service.exits.get())
            assertThrows(IllegalStateException::class.java) {
                ConnectionWorkerIpc.command(binder, ConnectionWorkerIpc.START_TRUST, 9, byteArrayOf(1, 99), events)
            }
            assertEquals(1, service.exits.get()); assertEquals(0, calls.get())
        } finally { controller.destroy() }
    }
    @Test fun t1151DefaultPrivateFactoryDoesNotNeedUsbOrGenerateIdentity() = runBlocking<Unit> {
        val controller = Robolectric.buildService(SavedTrustDefaultService::class.java).create()
        val events = Events()
        try {
            trust(bind(controller.get()), events)
            until { events.values.any { it is ConnectionWorkerEvent.Ended } }
            assertTrue(events.values.all { it is ConnectionWorkerEvent.SavedComputers || it is ConnectionWorkerEvent.Ended })
        } finally { controller.destroy() }
    }
}
