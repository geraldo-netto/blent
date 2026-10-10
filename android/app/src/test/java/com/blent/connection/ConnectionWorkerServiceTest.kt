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
import org.robolectric.shadows.ShadowProcess
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

class ConnectionProbeService : ConnectionWorkerService() {
    internal var factory: (ConnectionWorkerSpec, (ConnectionWorkerEvent) -> Unit) -> ConnectionWorker = { _, _ -> error("factory missing") }
    val exits = AtomicInteger()
    override fun createWorker(spec: ConnectionWorkerSpec, report: (ConnectionWorkerEvent) -> Unit) = factory(spec, report)
    override fun terminate() { exits.incrementAndGet() }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionWorkerServiceTest {
    private class Events : Binder() {
        val values = CopyOnWriteArrayList<ConnectionWorkerEvent>()
        var death: IBinder.DeathRecipient? = null
        var failLink = false
        var failUnlink = false
        var reject = false
        var gate: CountDownLatch? = null
        override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) {
            if (failLink) throw RemoteException("callback already dead")
            death = recipient
        }
        override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int): Boolean {
            if (failUnlink) error("unlink failed")
            death = null; return true
        }
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.EVENT, flags)
            assertEquals(7L, ConnectionWorkerIpc.generation(data))
            values.add(ConnectionWorkerCodec.event(ConnectionWorkerIpc.bytes(data))); ConnectionWorkerIpc.end(data)
            gate?.let { check(it.await(3, TimeUnit.SECONDS)) }
            return !reject
        }
    }
    private class Worker(private val phase: String = "", private val release: CompletableDeferred<Unit>? = null) : ConnectionWorker {
        val commands = CopyOnWriteArrayList<ConnectionWorkerCommand>()
        val entered = CompletableDeferred<Unit>()
        val cancelled = AtomicInteger()
        val retired = AtomicInteger()
        override suspend fun execute() {
            entered.complete(Unit)
            if (phase == "execute") error(phase)
            if (release != null) awaitCancellation()
        }
        override fun command(value: ConnectionWorkerCommand): Boolean { commands.add(value); return phase != "command" }
        override fun cancel() { cancelled.incrementAndGet(); if (phase == "cancel") error(phase) }
        override suspend fun retire() { retired.incrementAndGet(); release?.await(); if (phase == "retire") error(phase) }
    }
    private suspend fun until(condition: () -> Boolean) = withTimeout(2000) { while (!condition()) delay(1) }
    private fun bind(service: ConnectionWorkerService, revision: Long = 7) = service.onBind(Intent().putExtra("connection_generation", revision))
    private fun start(binder: IBinder, events: Events) = ConnectionWorkerIpc.command(binder, 1, 7,
        ConnectionWorkerCodec.spec(ConnectionWorkerSpec("usb")), events)
    private fun control() = ConnectionWorkerCommand.Revoke(ConnectionPeer.parse("ab".repeat(32)))

    @Test fun t884StopAndExitRemainResponsiveWhileNativeRetirementWaits() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); val events = Events(); val release = CompletableDeferred<Unit>()
        val worker = Worker(release = release); service.factory = { _, _ -> worker }
        try {
            val binder = bind(service); start(binder, events); withTimeout(2000) { worker.entered.await() }
            ConnectionWorkerIpc.command(binder, 2, 7, ConnectionWorkerCodec.command(control()))
            until { worker.commands.size == 1 }
            ConnectionWorkerIpc.command(binder, 3, 7); ConnectionWorkerIpc.command(binder, 3, 7)
            until { worker.cancelled.get() == 1 }; assertTrue(events.values.none { it is ConnectionWorkerEvent.Ended })
            assertThrows(IllegalStateException::class.java) { ConnectionWorkerIpc.command(binder, 4, 6) }
            assertEquals(0, service.exits.get()); ConnectionWorkerIpc.command(binder, 4, 7); assertEquals(1, service.exits.get())
            events.death!!.binderDied(); assertEquals(2, service.exits.get())
            release.complete(Unit); until { events.values.any { it is ConnectionWorkerEvent.Ended } }
            assertEquals(1, worker.retired.get()); assertEquals(1, worker.cancelled.get())
            assertEquals(android.app.Service.START_NOT_STICKY, service.onStartCommand(null, 0, 0))
            assertFalse(service.onUnbind(Intent()))
        } finally { release.complete(Unit); controller.destroy() }
        assertNull(events.death)
    }
    @Test fun t884FailureAtEachLifecycleBoundaryIsBoundedAndRetained() = runBlocking<Unit> {
        for (phase in listOf("create", "execute", "cancel", "retire")) {
            val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
            val service = controller.get(); val events = Events(); val worker = Worker(phase)
            service.factory = { _, _ -> if (phase == "create") error(phase); worker }
            try {
                start(bind(service), events); until { events.values.any { it is ConnectionWorkerEvent.Ended } }
                assertEquals(ConnectionWorkerEvent.Ended(phase), events.values.last())
                if (phase != "create") assertEquals(1, worker.retired.get())
            } finally { controller.destroy() }
        }
    }
    @Test fun t884BindCannotRetargetAnExistingProcessAndStartAdmitsOnlyOnce() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); val worker = Worker(); service.factory = { _, _ -> worker }
        try {
            assertThrows(IllegalArgumentException::class.java) { bind(service, 0) }
            val binder = bind(service); assertSame(binder, bind(service))
            assertThrows(IllegalStateException::class.java) { bind(service, 8) }
            val events = Events(); start(binder, events); until { events.values.any { it is ConnectionWorkerEvent.Ended } }
            assertThrows(IllegalStateException::class.java) { start(binder, events) }
            assertTrue(service.exits.get() > 0)
            assertThrows(IllegalArgumentException::class.java) { ConnectionWorkerIpc.command(binder, 99, 7) }
        } finally { controller.destroy() }
    }
    @Test fun t884FactoryStallDoesNotHoldBinderOrLoseReturnedNativeOwner() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val worker = Worker(); val events = Events()
        service.factory = { _, _ -> entered.countDown(); check(release.await(3, TimeUnit.SECONDS)); worker }
        try {
            val binder = bind(service); start(binder, events); assertTrue(entered.await(2, TimeUnit.SECONDS))
            ConnectionWorkerIpc.command(binder, 3, 7); ConnectionWorkerIpc.command(binder, 4, 7)
            assertEquals(1, service.exits.get()); release.countDown()
            until { worker.retired.get() == 1 }; assertEquals(1, worker.cancelled.get())
            assertFalse(worker.entered.isCompleted)
        } finally { release.countDown(); controller.destroy() }
    }
    @Test fun t884DeadCallbackCannotLeaveUnobservedServiceAlive() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); val opened = AtomicInteger()
        service.factory = { _, _ -> opened.incrementAndGet(); Worker() }
        try {
            start(bind(service), Events().apply { failLink = true })
            until { service.exits.get() > 0 }
            assertEquals(0, opened.get())
        } finally { controller.destroy() }
    }
    @Test fun t884RejectedEventAndControlDeliveryRequestProcessExit() = runBlocking<Unit> {
        for (phase in listOf("event", "command", "oversize")) {
            val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
            val service = controller.get(); val release = CompletableDeferred<Unit>(); val worker = Worker(phase, release)
            val events = Events().apply { reject = phase == "event" }
            service.factory = { _, report ->
                report(ConnectionWorkerEvent.Ended(if (phase == "oversize") "x".repeat(513) else null)); worker
            }
            try {
                val binder = bind(service); start(binder, events)
                if (phase == "command") ConnectionWorkerIpc.command(binder, 2, 7, ConnectionWorkerCodec.command(control()))
                until { service.exits.get() > 0 }
            } finally { release.complete(Unit); controller.destroy() }
        }
    }
    @Test fun t884DestroyAlwaysExitsEvenWhenUnlinkFailsAndDefaultBackendIsLazy() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); service.factory = { _, _ -> Worker() }; val events = Events()
        start(bind(service), events); until { events.values.any { it is ConnectionWorkerEvent.Ended } }
        events.failUnlink = true
        assertThrows(IllegalStateException::class.java) { controller.destroy() }; assertTrue(service.exits.get() > 0)
        val real = Robolectric.buildService(ConnectionWorkerService::class.java).create()
        try {
            val selected = AccessorySelection(1, "ab".repeat(32)).encode()
            val owned = real.get().createWorker(ConnectionWorkerSpec(selected)) {}
            owned.cancel(); owned.retire()
        } finally { real.destroy() }
        assertTrue(ShadowProcess.wasKilled(Process.myPid()))
    }
    @Test fun t854ProductionServiceFactoryDefersNativeLookupUntilExecution() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionWorkerService::class.java).create()
        val selected = AccessorySelection(1, "ab".repeat(32)).encode()
        val worker = controller.get().createWorker(ConnectionWorkerSpec(selected,
            io = ConnectionRunLimits(100, 100), nativeTimeoutMs = 100, timerMs = 1)) {}
        try {
            val result = runCatching { withTimeout(2000) { worker.execute() } }
            assertTrue(result.isFailure)
            assertFalse(result.exceptionOrNull() is TimeoutCancellationException)
        } finally { worker.cancel(); worker.retire(); controller.destroy() }
    }
    @Test fun t884CommandBacklogCannotGrowWhileFactoryIsBlocked() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val worker = Worker()
        service.factory = { _, _ -> entered.countDown(); check(release.await(3, TimeUnit.SECONDS)); worker }
        try {
            val binder = bind(service); start(binder, Events()); assertTrue(entered.await(2, TimeUnit.SECONDS))
            repeat(32) { ConnectionWorkerIpc.command(binder, 2, 7, ConnectionWorkerCodec.command(control())) }
            assertThrows(IllegalStateException::class.java) { ConnectionWorkerIpc.command(binder, 2, 7, ConnectionWorkerCodec.command(control())) }
            assertTrue(service.exits.get() > 0)
            release.countDown(); until { worker.retired.get() == 1 }
        } finally { release.countDown(); controller.destroy() }
    }
    @Test fun t884EventBacklogCannotGrowWhileCallbackIsBlocked() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); val release = CountDownLatch(1); val events = Events().apply { gate = release }
        service.factory = { _, report ->
            repeat(40) { report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING)) }
            Worker()
        }
        try {
            start(bind(service), events); until { service.exits.get() > 0 }
            release.countDown(); until { events.values.any { it is ConnectionWorkerEvent.State } }
        } finally { release.countDown(); controller.destroy() }
    }
    @Test fun t884StopCancellationFailureExitsEvenWhileExecuteCannotUnwind() = runBlocking<Unit> {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        val worker = object : ConnectionWorker {
            override suspend fun execute() = withContext(NonCancellable) { entered.complete(Unit); release.await() }
            override fun command(value: ConnectionWorkerCommand) = true
            override fun cancel() { error("cancel failed") }
            override suspend fun retire() {}
        }
        service.factory = { _, _ -> worker }
        try {
            val binder = bind(service); start(binder, Events()); withTimeout(2000) { entered.await() }
            ConnectionWorkerIpc.command(binder, ConnectionWorkerIpc.STOP, 7)
            until { service.exits.get() > 0 }
        } finally { release.complete(Unit); controller.destroy() }
    }
    @Test fun t884TrailingStartDataAndInvalidEnvelopeCannotAdmitWorker() {
        val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
        val service = controller.get(); val opened = AtomicInteger()
        service.factory = { _, _ -> opened.incrementAndGet(); Worker() }
        try {
            val binder = bind(service)
            for (mode in 0..2) {
                val data = Parcel.obtain()
                try {
                    data.writeInterfaceToken(ConnectionWorkerIpc.REQUEST); data.writeLong(7)
                    data.writeByteArray(ConnectionWorkerCodec.spec(ConnectionWorkerSpec("usb"))); data.writeStrongBinder(Events())
                    if (mode == 0) data.writeInt(999)
                    assertFalse(binder.transact(if (mode == 2) 99 else 1, data, null, if (mode == 1) 0 else 1))
                } finally { data.recycle() }
            }
            assertEquals(0, opened.get()); assertTrue(service.exits.get() > 0)
        } finally { controller.destroy() }
    }

}
