package com.blent.connection

import java.io.DataInputStream
import java.net.Socket
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

/** T1091: actual local listeners and stream workers; no USB or media devices.
 * Only a retained hint advances the live resource owner. Cleanup polls separately
 * so the deliberately faulty control cannot leave native test workers behind. */
internal class LocalWakeRig {
    private val hints = ArrayBlockingQueue<Unit>(32)
    val protocol = TabletSessionRig(capabilities = 1, timeout = 60000)
    val local = ConnectionLocalResources(protocol.tablet, 4096, wake = { hints.offer(Unit); Unit })
    private val sockets = ArrayList<Socket>()
    fun step() { local.advance(0); protocol.pump() }
    fun advanceUntil(ready: () -> Boolean) {
        repeat(64) {
            if (ready()) return
            hintStep()
        }
        error("T1091 bounded wake work did not settle")
    }
    fun hintStep() {
        assertNotNull("T1091 original native completion did not wake its resource owner", hints.poll(3, TimeUnit.SECONDS))
        step()
    }
    fun open(): LocalDisplayRoute {
        val offer = protocol.invitation()
        protocol.send(offer); step()
        advanceUntil { protocol.messages.any { it == ConnectionManagement.Prepared(offer.key) } }
        protocol.send(ConnectionManagement.Commit(offer.key))
        protocol.host.activate(1, 1); protocol.host.activate(2, 1); step()
        return checkNotNull(local.display())
    }
    fun connect(port: Int): Socket = Socket("127.0.0.1", port).apply {
        soTimeout = 3000
        sockets.add(this)
    }
    fun discardHints() { hints.clear() }
    fun close() {
        local.stop(0); sockets.forEach { it.close() }
        protocol.tablet.disconnect(); protocol.tablet.transportRetired()
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
        while (!local.retired()) {
            check(System.nanoTime() < deadline) { "T1091 original native cleanup did not complete" }
            local.advance(0); Thread.sleep(1)
        }
        assertFalse(local.quarantined())
    }
}

class ConnectionLocalWakeTest {
    @Test fun t1091LateOriginalWakeCannotRestartRetiredActor() = runBlocking {
        val setup = RuntimeSetup()
        val hints = ArrayList<() -> Unit>()
        val events = ArrayList<ConnectionWorkerEvent>()
        val worker = AndroidConnectionWorker(ConnectionWorkerSpec("fixture", timerMs = 1), { _, wake ->
            hints.add(wake)
            object : ConnectionSetup by setup {
                override fun stop() { setup.stop(); setup.gone = true }
            }
        }, events::add, { 0 })
        val execution = async { worker.execute() }
        try {
            withTimeout(3000) { while (hints.isEmpty()) yield() }
        } finally { worker.cancel(); execution.await(); worker.retire() }
        repeat(64) { hints.single()() }
        assertFalse(worker.command(ConnectionWorkerCommand.Revoke(setup.ticket.peer)))
        assertTrue(events.isEmpty()); assertEquals(1, hints.size)
    }

    @Test fun t1091BindAcceptAndDuplexProgressWakeWithoutTimerPolling() {
        val rig = LocalWakeRig()
        try {
            val route = rig.open()
            val socket = rig.connect(route.controlPort)
            rig.connect(route.displayPort)
            socket.getOutputStream().write(byteArrayOf(4, 5, 6))
            rig.advanceUntil { rig.protocol.payloads.sumOf { it.second.size } == 3 }
            assertArrayEquals(byteArrayOf(4, 5, 6), rig.protocol.payloads.flatMap { it.second.toList() }.toByteArray())
            rig.discardHints()
            assertTrue(rig.protocol.host.offer(1, byteArrayOf(7, 8, 9)))
            rig.protocol.hostDrain(); rig.step()
            rig.hintStep() // Exact stream write completion enables flush.
            rig.hintStep() // Flush completion returns the borrowed ring receipt.
            val received = ByteArray(3)
            DataInputStream(socket.getInputStream()).readFully(received)
            assertArrayEquals(byteArrayOf(7, 8, 9), received)
            assertNotNull(rig.local.display())
        } finally { rig.close() }
    }

    @Test fun t1091ProductionRunnerPassesOriginalHintAndRetainsCleanupAfterStop() {
        val hints = ArrayBlockingQueue<Unit>(32)
        val setup = RuntimeSetup(); val io = RunnerIo()
        val run = tabletConnectionRun(ConnectionWorkerSpec("fixture"), ConnectionSetupReady(io, setup.approval, 4096),
            { hints.offer(Unit); Unit })
        try {
            val session = run.endpoints() as TabletSession
            val grant = checkNotNull(session.nextFrame(0)); session.written(grant, 0)
            session.receive(ConnectionFrame(ConnectionWire.Header(ConnectionWire.CREDIT, ConnectionWire.MANAGEMENT,
                1, 1, 0, 0, 4096), byteArrayOf(), 0, 0), 0)
            val bytes = ConnectionManagement.encode(ConnectionManagement.Offer(LocalPumpRig().lease.key,
                ConnectionManagement.Credential(ByteArray(32)), ConnectionManagement.Display))
            session.receive(ConnectionFrame(ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.MANAGEMENT,
                1, 1, 1, bytes.size, 0), bytes, 0, bytes.size), 0)
            run.advance(0)
            assertNotNull("T1091 production composition discarded the original hint", hints.poll(3, TimeUnit.SECONDS))
            assertFalse(run.retired())
        } finally {
            run.stop(0)
            io.requests.forEach { io.complete(it, 0, java.io.IOException("fixture cancelled")) }
            io.gone = true
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
            while (!run.retired()) {
                check(System.nanoTime() < deadline) { "T1091 production local cleanup did not settle" }
                run.advance(1); Thread.sleep(1)
            }
        }
        assertNull(run.display())
    }
}
