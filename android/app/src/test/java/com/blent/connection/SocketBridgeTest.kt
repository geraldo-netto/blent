package com.blent.connection

import java.io.*
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import org.junit.Assert.*
import org.junit.Test

class SocketBridgeTest {
    private class NativeInput(private val bytes: ByteArray, private var zeros: Int = 0) : InputStream() {
        val closeRequested = AtomicBoolean()
        val release = CountDownLatch(1)
        var position = 0
        override fun read(): Int = error("Must use full USB read buffer")
        override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
            assertEquals(65536, length)
            if (zeros-- > 0) return 0
            if (position == bytes.size) { check(release.await(5, TimeUnit.SECONDS)); return -1 }
            val count = minOf(137, bytes.size - position)
            bytes.copyInto(buffer, offset, position, position + count); position += count
            return count
        }
        override fun close() { closeRequested.set(true); release.countDown() }
    }
    private class NativeOutput : OutputStream() {
        val bytes = ByteArrayOutputStream()
        val closed = AtomicBoolean()
        @Synchronized override fun write(value: Int) { bytes.write(value) }
        @Synchronized override fun write(buffer: ByteArray, offset: Int, count: Int) { bytes.write(buffer, offset, count) }
        @Synchronized fun snapshot(): ByteArray = bytes.toByteArray()
        override fun close() { closed.set(true) }
    }
    private fun eventually(condition: () -> Boolean) {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        while (!condition() && System.nanoTime() < deadline) Thread.sleep(2)
        assertTrue("Expected bounded retirement/progress", condition())
    }
    @Test fun t846LoopbackBridgeDisablesNagleInBothDirections() {
        SocketBridgeSockets.open().use { sockets ->
            assertEquals("T846: TLS and metadata must not wait on loopback Nagle/delayed ACK", listOf(true, true),
                listOf(sockets.client.tcpNoDelay, sockets.relay.tcpNoDelay))
        }
    }
    @Test fun t846NoDelaySetupFailureClosesEveryAcquiredSocket() {
        for (failClient in listOf(true, false)) {
            val injected = java.net.SocketException("Injected TCP_NODELAY failure")
            val client = object : Socket() {
                override fun setTcpNoDelay(enabled: Boolean) {
                    if (failClient) throw injected
                    super.setTcpNoDelay(enabled)
                }
            }
            val relay = object : Socket() {
                override fun setTcpNoDelay(enabled: Boolean) { throw injected }
            }
            val listener = object : ServerSocket() {
                override fun accept(): Socket = relay
            }
            try {
                val failure = assertThrows(java.net.SocketException::class.java) {
                    SocketBridgeSockets.open(listener) { client }
                }
                assertSame(injected, failure)
                assertTrue(client.isClosed); assertTrue(listener.isClosed)
                assertEquals(!failClient, relay.isClosed)
            } finally { client.close(); relay.close(); listener.close() }
        }
    }
    @Test fun t846BridgeTransfersBothDirectionsWithShortReadsAndUsbZeros() {
        val inbound = ByteArray(180123).apply { java.util.Random(846).nextBytes(this) }
        val outbound = ByteArray(160013).apply { java.util.Random(847).nextBytes(this) }
        val input = NativeInput(inbound, zeros = 3)
        val output = NativeOutput()
        val admission = SocketBridgeAdmission()
        val bridge = admission.open(input, output)
        try {
            bridge.start(); bridge.socket.soTimeout = 3000
            bridge.socket.getOutputStream().write(outbound)
            val received = ByteArray(inbound.size)
            DataInputStream(bridge.socket.getInputStream()).readFully(received)
            assertArrayEquals(inbound, received)
            eventually { output.snapshot().size == outbound.size }
            assertArrayEquals(outbound, output.snapshot())
            assertFalse(bridge.awaitTerminal(0)); assertNull(bridge.failure()); assertFalse(bridge.retired())
            assertThrows(IllegalStateException::class.java) { bridge.start() }
            val retained = NativeOutput()
            assertThrows(IllegalStateException::class.java) { admission.open(ByteArrayInputStream(byteArrayOf()), retained) }
            assertFalse(retained.closed.get())
        } finally { bridge.close() }
        eventually { bridge.retired() }
        assertTrue(input.closeRequested.get()); assertTrue(output.closed.get())
        assertTrue(bridge.awaitTerminal(0)); bridge.close()
    }
    @Test fun t846StopRequestDoesNotClaimBlockedWorkerRetirement() {
        val input = NativeInput(byteArrayOf())
        val bridge = SocketBridgeAdmission().open(input, NativeOutput())
        bridge.start()
        bridge.requestStop()
        assertTrue(bridge.awaitTerminal(10)); assertFalse(bridge.retired())
        bridge.close(); eventually { bridge.retired() }
    }
    @Test fun t846GracefulOutputDrainRequiresActualEofAndThreadExit() {
        val input = NativeInput(byteArrayOf())
        val output = NativeOutput()
        val bridge = SocketBridgeAdmission().open(input, output)
        assertFalse(bridge.awaitOutputRetired(0))
        for (timeout in listOf(-1L, 60001L)) assertThrows(IllegalArgumentException::class.java) { bridge.awaitOutputRetired(timeout) }
        try {
            bridge.start(); assertFalse(bridge.awaitOutputRetired(0))
            bridge.socket.getOutputStream().write(byteArrayOf(1, 2, 3))
            bridge.socket.shutdownOutput()
            assertTrue(bridge.awaitOutputRetired(2000))
            assertArrayEquals(byteArrayOf(1, 2, 3), output.snapshot())
        } finally { bridge.close() }
        eventually { bridge.retired() }
    }
    @Test fun t846IncomingEofCannotPretendOutgoingReachedRelayEof() {
        val inboundEof = CountDownLatch(1)
        val writing = CountDownLatch(1)
        val releaseWrite = CountDownLatch(1)
        val input = object : InputStream() {
            override fun read(): Int { check(inboundEof.await(5, TimeUnit.SECONDS)); return -1 }
            override fun close() { inboundEof.countDown() }
        }
        val output = object : OutputStream() {
            override fun write(value: Int) { writing.countDown(); check(releaseWrite.await(5, TimeUnit.SECONDS)) }
        }
        val bridge = SocketBridgeAdmission().open(input, output)
        try {
            bridge.start(); bridge.socket.getOutputStream().write(1)
            assertTrue(writing.await(2, TimeUnit.SECONDS))
            inboundEof.countDown(); assertTrue(bridge.awaitTerminal(2000))
            releaseWrite.countDown()
            assertFalse(bridge.awaitOutputRetired(2000))
            assertNull(bridge.failure())
        } finally { releaseWrite.countDown(); bridge.close() }
        eventually { bridge.retired() }
    }
    @Test fun t846CompletedCloseDoesNotClaimStuckNativeReadRetired() {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val input = object : InputStream() {
            override fun read(): Int {
                entered.countDown(); check(release.await(5, TimeUnit.SECONDS)); return -1
            }
            override fun close() {} // Models a driver read not cancelled by close.
        }
        val bridge = SocketBridgeAdmission().open(input, NativeOutput())
        try {
            bridge.start(); assertTrue(entered.await(2, TimeUnit.SECONDS))
            bridge.close(); assertFalse(bridge.retired())
        } finally { release.countDown(); bridge.close() }
        eventually { bridge.retired() }
    }
    @Test fun t846NativeCloseCompletionIsPartOfRetirement() {
        val closing = CountDownLatch(1)
        val release = CountDownLatch(1)
        val input = object : ByteArrayInputStream(byteArrayOf()) {
            override fun close() { closing.countDown(); check(release.await(5, TimeUnit.SECONDS)) }
        }
        val bridge = SocketBridgeAdmission().open(input, NativeOutput())
        bridge.start()
        val closer = Thread { bridge.close() }
        try {
            closer.start(); assertTrue(closing.await(2, TimeUnit.SECONDS))
            assertFalse(bridge.retired())
        } finally { release.countDown(); closer.join(3000) }
        assertFalse(closer.isAlive); eventually { bridge.retired() }
    }
    @Test fun t846InvalidReadCountsAndNativeFailuresRequestRetirement() {
        for (count in listOf(-2, 65537, Int.MAX_VALUE)) {
            val input = object : InputStream() {
                override fun read(): Int = error("Unused")
                override fun read(buffer: ByteArray, offset: Int, length: Int): Int = count
            }
            val bridge = SocketBridgeAdmission().open(input, NativeOutput())
            try { bridge.start(); assertTrue(bridge.awaitTerminal(2000)); assertNotNull(bridge.failure()) }
            finally { bridge.close() }
            eventually { bridge.retired() }
        }
        val input = object : InputStream() { override fun read(): Int = throw IOException("Native failed") }
        val bridge = SocketBridgeAdmission().open(input, NativeOutput())
        try { bridge.start(); assertTrue(bridge.awaitTerminal(2000)); assertTrue(bridge.failure() is IOException) }
        finally { bridge.close() }
        eventually { bridge.retired() }
    }
    @Test fun t846EofAndOutputFailureSignalTerminal() {
        val eof = SocketBridgeAdmission().open(ByteArrayInputStream(byteArrayOf()), NativeOutput())
        try { eof.start(); assertTrue(eof.awaitTerminal(2000)); assertNull(eof.failure()) } finally { eof.close() }
        eventually { eof.retired() }
        val input = NativeInput(byteArrayOf())
        val output = object : OutputStream() { override fun write(value: Int) { throw IOException("Write failed") } }
        val bridge = SocketBridgeAdmission().open(input, output)
        try {
            bridge.start(); bridge.socket.getOutputStream().write(42)
            assertTrue(bridge.awaitTerminal(2000)); assertTrue(bridge.failure() is IOException)
        } finally { bridge.close() }
        eventually { bridge.retired() }
    }
    @Test fun t846ClosingAnUnstartedBridgePreventsStartAndIsRetired() {
        val bridge = SocketBridgeAdmission().open(NativeInput(byteArrayOf()), NativeOutput())
        bridge.close(); assertTrue(bridge.retired())
        assertThrows(IllegalStateException::class.java) { bridge.start() }
        assertTrue(bridge.retired())
        for (timeout in listOf(-1L, Long.MIN_VALUE, 60001L, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { bridge.awaitTerminal(timeout) }
        }
    }
    @Test fun t846ThreadSetupAndPartialStartupRetainLifecycleHandle() {
        for (startFailure in listOf(false, true)) {
            var count = 0
            val admission = SocketBridgeAdmission(threads = { name, task ->
                count++
                if (count == 2 && !startFailure) throw IllegalStateException("Thread creation rejected")
                if (count == 2) object : Thread(task, name) { override fun start() { throw IllegalStateException("Thread start rejected") } }
                else Thread(task, name)
            })
            val bridge = admission.open(NativeInput(byteArrayOf()), NativeOutput())
            assertThrows(IllegalStateException::class.java) { bridge.start() }
            assertNotNull(bridge.failure()); assertFalse(bridge.retired())
            bridge.close(); eventually { bridge.retired() }
        }
    }
    @Test fun t846SetupFailureClosesTransferredStreamsAndPreservesOriginalError() {
        val input = NativeInput(byteArrayOf())
        val output = NativeOutput()
        val failed = SocketBridgeAdmission(sockets = { throw IOException("Socket setup failed") })
        assertThrows(IOException::class.java) { failed.open(input, output) }
        assertTrue(input.closeRequested.get()); assertTrue(output.closed.get())
        val badClose = object : InputStream() {
            override fun read(): Int = -1
            override fun close() { throw IOException("Close failure") }
        }
        val error = assertThrows(IOException::class.java) {
            SocketBridgeAdmission(sockets = { throw IOException("Original") }).open(badClose, NativeOutput())
        }
        assertEquals("Original", error.message); assertEquals(1, error.suppressed.size)
    }
    @Test fun t846RepeatedExceptionInstanceCannotAbortCleanup() {
        val failure = IOException("Shared native failure")
        val input = object : InputStream() {
            override fun read(): Int = -1
            override fun close() { throw failure }
        }
        val output = NativeOutput()
        val observed = assertThrows(IOException::class.java) {
            SocketBridgeAdmission(sockets = { throw failure }).open(input, output)
        }
        assertSame(failure, observed); assertTrue(output.closed.get())
        val badOutput = object : OutputStream() {
            override fun write(value: Int) {}
            override fun close() { throw failure }
        }
        val bridge = SocketBridgeAdmission().open(input, badOutput)
        assertSame(failure, assertThrows(IOException::class.java) { bridge.close() })
        assertFalse(bridge.retired())
    }
    @Test fun t846WrongLoopbackPeerIsRejectedAndClosed() {
        val first = SocketBridgeSockets.open()
        val second = SocketBridgeSockets.open()
        val wrong = SocketBridgeSockets(first.client, second.relay)
        try {
            assertThrows(IllegalStateException::class.java) {
                SocketBridgeAdmission(sockets = { wrong }).open(NativeInput(byteArrayOf()), NativeOutput())
            }
            assertTrue(first.client.isClosed); assertTrue(second.relay.isClosed)
            assertThrows(IllegalStateException::class.java) { SocketBridgeSockets(Socket(), Socket()).verify() }
        } finally { first.close(); second.close() }
    }
    @Test fun t846ListenerFailureRollsBackClientAndListener() {
        val listener = object : ServerSocket() {
            override fun accept(): Socket { throw IOException("Accept rejected") }
        }
        assertThrows(IOException::class.java) { SocketBridgeSockets.open(listener) }
        assertTrue(listener.isClosed)
    }
    @Test fun t846CloseFailuresAttemptEveryResourceAndNeverClaimRetirement() {
        val input = object : InputStream() {
            override fun read(): Int = -1
            override fun close() { throw IOException("Input close failed") }
        }
        val output = object : OutputStream() {
            var attempted = false
            override fun write(value: Int) {}
            override fun close() { attempted = true; throw IOException("Output close failed") }
        }
        val bridge = SocketBridgeAdmission().open(input, output)
        val error = assertThrows(IOException::class.java) { bridge.close() }
        assertEquals(1, error.suppressed.size); assertTrue(output.attempted)
        assertSame(error, bridge.failure()); assertFalse(bridge.retired()); bridge.close()
    }
}
