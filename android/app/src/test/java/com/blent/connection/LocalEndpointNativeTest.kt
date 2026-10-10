package com.blent.connection

import java.io.IOException
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test

private fun awaitNative(condition: () -> Boolean) {
    val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
    while (!condition()) { check(System.nanoTime() < deadline) { "T906 native fixture timed out" }; Thread.sleep(1) }
}
private fun CountDownLatch.bounded() { check(await(3, TimeUnit.SECONDS)) { "T906 latch timed out" } }

internal class LocalNativeFixture : LocalEndpointNative {
    val bindEntered = CountDownLatch(1)
    val bindRelease = CountDownLatch(1)
    val acceptEntered = CountDownLatch(1)
    val acceptRelease = CountDownLatch(1)
    val closeEntered = CountDownLatch(1)
    val closeRelease = CountDownLatch(1)
    val io = RunnerIo()
    @Volatile var closed = false
    @Volatile var closeFailure = false
    var port = 23456
    override fun bind(): Int { bindEntered.countDown(); bindRelease.bounded(); return port }
    override fun accept(): ConnectionIo { acceptEntered.countDown(); acceptRelease.bounded(); return io }
    override fun close() {
        closeEntered.countDown(); closeRelease.bounded()
        if (closeFailure) throw IOException("fixture close failed")
        io.stop(); io.gone = true; closed = true
    }
    override fun retired() = closed
    fun release() { bindRelease.countDown(); acceptRelease.countDown(); closeRelease.countDown() }
}

class LocalEndpointNativeTest {
    @Test fun t906BoundPortPublishesWithoutBlockingAndAcceptedIoTransfersOnce() {
        val native = LocalNativeFixture()
        val owner = LocalEndpointOwner(native, 0, ConnectionRunLimits(10, 5))
        try {
            native.bindEntered.bounded(); assertNull(owner.port()); assertNull(owner.takeIo())
            native.bindRelease.countDown(); awaitNative { owner.port() != null }
            assertEquals(23456, owner.port()); owner.advance(1000) // Idle accept before admission.
            assertNull(owner.failure()); owner.admit(1000)
            native.acceptRelease.countDown(); awaitNative { native.acceptEntered.count == 0L }
            var io: ConnectionIo? = null
            awaitNative { owner.takeIo().also { if (it != null) io = it } != null }
            assertSame(native.io, io); assertNull(owner.takeIo())
            owner.advance(2000); assertNull(owner.failure()); assertFalse(owner.retired())
            owner.stop(2000); assertNull(owner.port()); native.closeRelease.countDown()
            awaitNative { owner.retired() }; assertFalse(owner.quarantined())
        } finally { owner.stop(2000); native.release(); awaitNative { owner.retired() } }
    }
    @Test fun t906BindingDeadlineRetainsWorkerAfterCloserAlreadyExited() {
        val native = LocalNativeFixture(); native.closeRelease.countDown()
        val owner = LocalEndpointOwner(native, 0, ConnectionRunLimits(10, 5))
        try {
            native.bindEntered.bounded(); owner.advance(10); native.closeEntered.bounded()
            awaitNative { native.closed }; assertFalse(owner.retired())
            owner.advance(15); assertTrue(owner.quarantined()); assertNull(owner.port())
            native.bindRelease.countDown(); awaitNative { owner.retired() }
            assertFalse(owner.quarantined()); assertNull(owner.takeIo())
        } finally { native.release(); owner.stop(15) }
    }
    @Test fun t906AcceptDeadlineAndStuckCloseDoNotReleaseOwnership() {
        val native = LocalNativeFixture(); native.bindRelease.countDown()
        val owner = LocalEndpointOwner(native, 0, ConnectionRunLimits(10, 5))
        try {
            awaitNative { owner.port() != null }; owner.admit(1); owner.advance(10)
            assertNull(owner.failure()); owner.advance(11); native.closeEntered.bounded()
            owner.advance(16); assertTrue(owner.quarantined()); assertFalse(owner.retired())
            native.acceptRelease.countDown(); assertFalse(owner.retired())
            native.closeRelease.countDown(); awaitNative { owner.retired() }
        } finally { native.release(); owner.stop(16) }
    }
    @Test fun t906CloseFailureAndInvalidNativePortRemainExplicit() {
        val native = LocalNativeFixture(); native.port = 0; native.closeFailure = true; native.release()
        val owner = LocalEndpointOwner(native, 0, ConnectionRunLimits(10, 5))
        awaitNative { owner.failure() != null }; native.closeEntered.bounded()
        owner.advance(5); awaitNative { owner.quarantined() }
        assertNull(owner.port()); assertNull(owner.takeIo()); assertFalse(owner.retired())
    }
    @Test fun t906WorkerCreationFailureAndBadClockNeverGrantEndpoint() {
        val native = LocalNativeFixture(); native.release()
        val owner = LocalEndpointOwner(native, 0, threads = { _, _ -> throw IOException("thread unavailable") })
        assertNotNull(owner.failure()); assertNull(owner.port()); assertFalse(owner.retired())
        for (now in listOf(Long.MIN_VALUE, -1, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { owner.advance(now) }
        }
        owner.advance(6000); assertTrue(owner.quarantined())
        assertThrows(IllegalStateException::class.java) { owner.admit(6000) }
        assertThrows(IllegalArgumentException::class.java) { owner.stop(5999) }
    }
    @Test fun t906NativeLoopbackAllocatesLocalPortAndClosesActualIoWorkers() {
        val native = LocalSocketEndpoint()
        val owner = LocalEndpointOwner(native, 0)
        val socket = Socket()
        try {
            awaitNative { owner.port() != null }; val port = checkNotNull(owner.port())
            owner.admit(1); socket.connect(InetSocketAddress("127.0.0.1", port), 1000)
            var io: ConnectionIo? = null
            awaitNative { owner.takeIo().also { if (it != null) io = it } != null }
            val read = ConnectionIoRequest.Read(ConnectionBuffer(ByteArray(8), 0, 8))
            assertTrue(io!!.submit(read)); socket.getOutputStream().write(byteArrayOf(1, 2, 3))
            var completion: ConnectionIoCompletion? = null
            awaitNative { io!!.poll().also { if (it != null) completion = it } != null }
            assertSame(read, completion!!.request); assertEquals(3, completion!!.count)
            assertArrayEquals(byteArrayOf(1, 2, 3), read.loan.bytes.copyOf(3))
            assertThrows(IllegalStateException::class.java) { native.bind() }
            assertThrows(IllegalStateException::class.java) { native.accept() }
        } finally { owner.stop(2); socket.close(); awaitNative { owner.retired() } }
    }
    @Test fun t906BackendCannotRetireWhileBindCallRemainsInFlight() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val listener = object : ServerSocket() {
            override fun bind(endpoint: java.net.SocketAddress?, backlog: Int) { entered.countDown(); release.bounded() }
        }
        val backend = LocalSocketEndpoint(listener = { listener })
        val worker = Thread { try { backend.bind() } catch (_: Exception) {} }
        worker.start(); entered.bounded()
        try {
            backend.close()
            assertFalse("T906 close return cannot retire an in-flight bind", backend.retired())
        } finally { release.countDown(); worker.join(3000); assertFalse(worker.isAlive) }
    }
    @Test fun t906UnreturnedIoFactoryOwnershipStaysQuarantined() {
        val backend = LocalSocketEndpoint(ioFactory = { throw IOException("factory acquired unknown work") })
        val port = backend.bind(); val peer = Socket("127.0.0.1", port)
        try { assertThrows(IOException::class.java) { backend.accept() } }
        finally { peer.close(); backend.close() }
        assertFalse("T906 throwing factory cannot prove partial native retirement", backend.retired())
    }
    @Test fun t906FailedIoCloseCannotBecomeBackendRetirement() {
        val io = RunnerIo().apply { gone = true; closeError = IOException("native close failed") }
        val backend = LocalSocketEndpoint(ioFactory = { io })
        val port = backend.bind(); val peer = Socket("127.0.0.1", port)
        try { assertSame(io, backend.accept()) }
        finally { peer.close(); backend.close() }
        assertFalse("T906 I/O thread exit is not successful resource closure", backend.retired())
    }
    @Test fun t906FailedStopOfIoCreatedAfterCancellationCannotRetire() {
        val io = object : ConnectionIo by (RunnerIo().apply { gone = true }) {
            override fun stop() { throw IOException("late stop failed") }
        }
        lateinit var backend: LocalSocketEndpoint
        backend = LocalSocketEndpoint(ioFactory = { backend.close(); io })
        val port = backend.bind(); val peer = Socket("127.0.0.1", port)
        try { assertThrows(IOException::class.java) { backend.accept() } }
        finally { peer.close(); backend.close() }
        assertFalse("T906 failed late stop has unresolved native ownership", backend.retired())
    }

    @Test fun t906LateListenersAndAcceptedSocketsCloseWithoutEscapingOwnership() {
        lateinit var backend: LocalSocketEndpoint
        val listener = ServerSocket()
        backend = LocalSocketEndpoint(listener = { backend.close(); listener })
        assertThrows(IllegalStateException::class.java) { backend.bind() }
        assertTrue(listener.isClosed); assertTrue(backend.retired())
        val peer = Socket()
        val server = object : ServerSocket() {
            override fun accept(): Socket { backend.close(); return peer }
        }
        backend = LocalSocketEndpoint(listener = { server }); backend.bind()
        assertThrows(IllegalStateException::class.java) { backend.accept() }
        assertTrue(peer.isClosed); assertTrue(server.isClosed); assertTrue(backend.retired())
    }
    @Test fun t906LateNativeCloseFailureAndUnreturnedListenerRemainQuarantined() {
        lateinit var backend: LocalSocketEndpoint
        val listener = object : ServerSocket() { override fun close() { super.close(); throw IOException("close failed") } }
        backend = LocalSocketEndpoint(listener = { backend.close(); listener })
        assertThrows(IOException::class.java) { backend.bind() }; assertFalse(backend.retired())
        val failed = LocalSocketEndpoint(listener = { throw IOException("unknown partial listener") })
        assertThrows(IOException::class.java) { failed.bind() }; failed.close(); assertFalse(failed.retired())
    }
    @Test fun t906NonLoopbackAndUnboundAcceptNeverCreateIo() {
        val fake = object : Socket() {
            override fun getInetAddress() = java.net.InetAddress.getByAddress(byteArrayOf(192.toByte(), 0, 2, 1))
        }
        val server = object : ServerSocket() { override fun accept() = fake }
        val backend = LocalSocketEndpoint(listener = { server }, ioFactory = { error("must not construct I/O") })
        assertThrows(IllegalStateException::class.java) { backend.accept() }
        backend.bind(); assertThrows(IllegalStateException::class.java) { backend.accept() }
        backend.close(); backend.close(); assertTrue(backend.retired()); assertTrue(fake.isClosed)
    }
    @Test fun t906WakeFailureStopsNativeWorkAndAllInvalidPortsReject() {
        val native = LocalNativeFixture(); native.release()
        val owner = LocalEndpointOwner(native, 0, wake = { throw IOException("wake failed") })
        awaitNative { owner.retired() }; assertNotNull(owner.failure()); assertNull(owner.port())
        val random = java.util.Random(906)
        val ports = listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE) + List(64) { random.nextInt() or Int.MIN_VALUE }
        for (port in ports) {
            val bad = LocalNativeFixture(); bad.port = port; bad.release()
            val rejected = LocalEndpointOwner(bad, 0)
            awaitNative { rejected.retired() }
            assertNotNull(rejected.failure()); assertNull(rejected.port()); assertNull(rejected.takeIo())
        }
    }

    @Test fun t906SuccessfulStopOfLateIoWaitsForActualWorkerRetirement() {
        val io = RunnerIo()
        lateinit var backend: LocalSocketEndpoint
        backend = LocalSocketEndpoint(ioFactory = { backend.close(); io })
        val peer = Socket("127.0.0.1", backend.bind())
        try { assertThrows(IllegalStateException::class.java) { backend.accept() } }
        finally { peer.close(); backend.close() }
        assertTrue(io.stopping); assertFalse(backend.retired())
        io.gone = true; assertTrue(backend.retired())
    }

}
