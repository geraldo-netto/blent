package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.net.Socket
import java.net.SocketTimeoutException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class SocketConnectionSetupTest {
    private fun backend(socket: Socket, clock: () -> Long = System::nanoTime) =
        SocketConnectionSetup(socket, SocketSetupFixture::identity, clock = clock)
    private fun rejected(body: () -> Unit) { assertThrows(Exception::class.java) { body() } }
    private fun until(condition: () -> Boolean) {
        val end = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        while (!condition()) { check(System.nanoTime() < end); Thread.sleep(1) }
    }
    @Test fun t886StockTlsSetupNegotiatesThenTransfersOnlyFramedIo() {
        val listener = SocketSetupFixture.listener(); val pool = Executors.newSingleThreadExecutor()
        val server = pool.submit { SocketSetupFixture.serve(listener) }
        val socket = Socket("127.0.0.1", listener.localPort)
        val backend = backend(socket)
        var io: ConnectionIo? = null
        try {
            val local = backend.localIdentity(); assertArrayEquals(SocketSetupFixture.bytes("tablet.der"), local.certificate())
            assertArrayEquals(local.certificate(), backend.localIdentity().certificate())
            val peer = backend.discover(3000)
            assertArrayEquals(SocketSetupFixture.bytes("host.der"), peer.certificate())
            rejected { backend.authenticate(peer) }
            backend.ready(peer, 3000); rejected { backend.ready(peer, 3000) }
            assertEquals(64, backend.authenticate(peer).fingerprint.length)
            assertEquals(4096, backend.negotiate(4096, 3000))
            val owned = backend.io {}; io = owned
            rejected { backend.io {} }; rejected { backend.negotiate(4096, 3000) }
            val write = ConnectionIoRequest.Write(ConnectionBuffer(byteArrayOf(42), 0, 1))
            assertTrue(owned.submit(write)); until { owned.poll()?.request === write }
            val flush = ConnectionIoRequest.Flush(ConnectionFlush())
            assertTrue(owned.submit(flush)); until { owned.poll()?.request === flush }
            val bytes = ByteArray(1); val read = ConnectionIoRequest.Read(ConnectionBuffer(bytes, 0, 1))
            assertTrue(owned.submit(read)); until { owned.poll()?.request === read }
            assertEquals(43, bytes[0].toInt())
            server.get(3, TimeUnit.SECONDS)
        } finally {
            io?.stop(); backend.close(); backend.close()
            until { io?.retired() != false && backend.retired() }
            listener.close(); pool.shutdownNow(); assertTrue(pool.awaitTermination(3, TimeUnit.SECONDS))
        }
        rejected { backend.localIdentity() }
    }
    @Test fun t886FragmentedDiscoveryRejectsDuplicateAndUnapprovedIdentity() {
        val socket = SetupMemorySocket().apply { maximumRead = 1 }
        val backend = backend(socket)
        try {
            val peer = backend.discover(3000)
            assertTrue(socket.output.size() > 0)
            rejected { backend.discover(3000) }
            rejected { backend.ready(ConnectionIdentity(SocketSetupFixture.bytes("stranger.der")), 1000) }
            rejected { backend.negotiate(4096, 1000) }; rejected { backend.io {} }
            rejected { backend.ready(peer, 1000) } // No readiness bytes.
        } finally { backend.close() }
        assertTrue(socket.closed); assertTrue(backend.retired())
    }
    @Test fun t886MalformedAndTruncatedPublicCertificatesRejectBeforeReply() {
        val valid = SocketSetupFixture.hello()
        val badCertificate = byteArrayOf(1, 2, 3)
        val records = listOf(byteArrayOf(), valid.copyOf(3), valid.copyOf(valid.size - 1),
            ConnectionHello.encode(true, 3) + badCertificate,
            ConnectionHello.encode(true, SocketSetupFixture.bytes("host.der").size + 1) + SocketSetupFixture.bytes("host.der") + 1)
        for (bytes in records) {
            val socket = SetupMemorySocket(bytes); val backend = backend(socket)
            try { rejected { backend.discover(1000) }; assertEquals(0, socket.output.size()) }
            finally { backend.close() }
        }
    }
    @Test fun t886AbsoluteReadDeadlineAndNativeInvalidCountsNeverExtendSetup() {
        for (result in listOf(-1, 0, Int.MAX_VALUE)) {
            val socket = SetupMemorySocket().apply { readResult = result }; val backend = backend(socket)
            try { rejected { backend.discover(1000) } } finally { backend.close() }
        }
        val time = AtomicLong(); val socket = SetupMemorySocket().apply {
            maximumRead = 1; afterRead = { time.addAndGet(1000000); Unit }
        }
        val backend = backend(socket, time::get)
        try { assertThrows(SocketTimeoutException::class.java) { backend.discover(2) } }
        finally { backend.close() }
    }
    @Test fun t886TimeoutBoundsAndWrongReadinessReject() {
        for (timeout in listOf(Long.MIN_VALUE, -1, 0, 60001, Long.MAX_VALUE)) {
            val backend = backend(SetupMemorySocket())
            try { rejected { backend.discover(timeout) } } finally { backend.close() }
        }
        val backend = backend(SetupMemorySocket(SocketSetupFixture.hello() + ByteArray(ConnectionReady.BYTES)))
        try {
            val peer = backend.discover(1000)
            rejected { backend.ready(peer, 600001) }
            rejected { backend.ready(peer, 1000) }
        } finally { backend.close() }
    }
    @Test fun t886NativeCloseFailureCannotEstablishRetirement() {
        val socket = SetupMemorySocket().apply { closeFailure = true }
        val backend = backend(socket)
        rejected { backend.close() }
        assertFalse("Failed native close is not retirement evidence", backend.retired())
        backend.close()
    }
    @Test fun t886ClosePreservesOriginalIncludingIdenticalCleanupFailures() {
        for (same in listOf(false, true)) {
            val original = IllegalStateException("socket close")
            val cleanup = if (same) original else IllegalArgumentException("bridge close")
            val socket = object : Socket() { override fun close() { throw original } }
            val lifetime = object : ConnectionSetupLifetime {
                override fun close() { throw cleanup }
                override fun retired() = false
            }
            val backend = SocketConnectionSetup(socket, SocketSetupFixture::identity, lifetime)
            assertSame(original, runCatching { backend.close() }.exceptionOrNull())
            assertEquals(if (same) 0 else 1, original.suppressed.size)
            assertFalse(backend.retired())
        }
        val lifetime = object : ConnectionSetupLifetime {
            override fun close() { error("bridge alone") }
            override fun retired() = false
        }
        val backend = SocketConnectionSetup(SetupMemorySocket(), SocketSetupFixture::identity, lifetime)
        assertEquals("bridge alone", runCatching { backend.close() }.exceptionOrNull()?.message)
        assertFalse(backend.retired())
    }
    @Test fun t886LateTlsCloseFailureRetainsOwnership() {
        val late = SetupMemorySocket().apply { closeFailure = true }
        val raw = SetupMemorySocket(SocketSetupFixture.hello() + ConnectionReady.encode(true))
        lateinit var backend: SocketConnectionSetup
        backend = SocketConnectionSetup(raw, SocketSetupFixture::identity, secure = { _, _, _ -> backend.close(); late })
        val peer = backend.discover(1000)
        backend.ready(peer, 1000)
        rejected { backend.authenticate(peer) }
        assertTrue(late.closed); assertFalse(backend.retired())
    }
    @Test fun t886RetirementIncludesUnderlyingBridgeAndLateTlsResult() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val late = SetupMemorySocket(); val raw = SetupMemorySocket(SocketSetupFixture.hello() + ConnectionReady.encode(true))
        var bridgeRetired = false
        val lifetime = object : ConnectionSetupLifetime {
            override fun close() {}
            override fun retired() = bridgeRetired
        }
        val backend = SocketConnectionSetup(raw, SocketSetupFixture::identity, lifetime, secure = { _, _, _ ->
            entered.countDown(); check(release.await(3, TimeUnit.SECONDS)); late
        })
        val peer = backend.discover(1000); backend.ready(peer, 1000)
        val pool = Executors.newSingleThreadExecutor()
        val result = pool.submit<Boolean> { runCatching { backend.authenticate(peer) }.isFailure }
        try {
            assertTrue(entered.await(2, TimeUnit.SECONDS)); backend.close()
            assertFalse(backend.retired()); release.countDown()
            assertTrue(result.get(2, TimeUnit.SECONDS)); assertTrue(late.closed)
            bridgeRetired = true; assertTrue(backend.retired())
        } finally { release.countDown(); backend.close(); pool.shutdownNow(); assertTrue(pool.awaitTermination(3, TimeUnit.SECONDS)) }
    }
}
