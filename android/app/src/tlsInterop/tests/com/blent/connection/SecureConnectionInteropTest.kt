package com.blent.connection

import java.io.File
import java.net.Socket
import java.nio.file.Files
import java.security.KeyFactory
import java.security.KeyStore
import java.security.cert.CertificateFactory
import java.security.spec.PKCS8EncodedKeySpec
import java.util.concurrent.TimeUnit
import javax.net.ssl.KeyManagerFactory
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** T846: real maintained Android adapter against the maintained Rust accept path.
 * Runs only in tlsInteropTest, required by scripts/tests/test_secure_connection_interop.py. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class SecureConnectionInteropTest {
    private fun fixture(name: String) = javaClass.classLoader!!
        .getResourceAsStream("connection-tls/$name")!!.use { it.readBytes() }

    private fun client(peer: String): SecureConnection {
        val certificate = CertificateFactory.getInstance("X.509").generateCertificate(fixture("tablet.der").inputStream())
        val privateKey = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(fixture("tablet.pk8")))
        val store = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null)
            setKeyEntry("tablet", privateKey, charArrayOf(), arrayOf(certificate))
        }
        val keys = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm()).apply { init(store, charArrayOf()) }
        return SecureConnection(keys.keyManagers, fixture("$peer.der"))
    }

    private fun port(process: Process, ready: File): Int {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
        while (System.nanoTime() < deadline) {
            check(process.isAlive) { "Rust fixture exited before readiness" }
            if (ready.isFile && ready.length() in 2..6) {
                val value = ready.readText().trim().toIntOrNull()
                if (value != null && value in 1..65535) return value
            }
            Thread.sleep(10)
        }
        error("Rust fixture readiness deadline")
    }

    private fun trial(peer: String = "host", mode: String = "accept", verified: (Socket) -> Unit = {}): Boolean {
        val directory = Files.createTempDirectory("blent-t846-interop-").toFile()
        try {
            listOf("host.der", "host.pk8", "tablet.der").forEach { File(directory, it).writeBytes(fixture(it)) }
            return runPeer(directory, peer, mode, verified)
        } finally { directory.deleteRecursively() }
    }

    private fun runPeer(directory: File, peer: String, mode: String, verified: (Socket) -> Unit): Boolean {
        val executable = checkNotNull(System.getProperty("blent.tls.fixture")) { "Missing Rust TLS fixture" }
        val ready = File(directory, "ready")
        val log = File(directory, "fixture.log")
        val process = ProcessBuilder(executable, directory.absolutePath, ready.absolutePath, mode)
            .redirectErrorStream(true).redirectOutput(log).start()
        try {
            val socket = Socket("127.0.0.1", port(process, ready)).apply { soTimeout = 3000 }
            var admitted = false
            val result = runCatching {
                fixtureStream(socket, peer, mode).use { secure ->
                    verified(secure)
                    admitted = true
                    if (!resourceMode(mode)) rawExchange(secure)
                }
            }
            if (peer != "host" || mode == "wrong-preface") assertFalse("Unapproved stream escaped authentication gate", admitted)
            if (result.isFailure) assertTrue(socket.isClosed)
            assertTrue("Rust fixture failed to retire: ${log.readText()}", process.waitFor(5, TimeUnit.SECONDS))
            if (mode == "wrong-preface") assertEquals("Fault injector must complete TLS and write bad preface", 0, process.exitValue())
            if (resourceMode(mode)) result.getOrThrow()
            return result.isSuccess && process.exitValue() == 0
        } finally {
            if (process.isAlive) process.destroyForcibly()
            check(process.waitFor(2, TimeUnit.SECONDS)) { "Rust fixture still owns resources" }
        }
    }

    private fun resourceMode(mode: String): Boolean = mode == "session" || mode == "setup"
    private fun fixtureStream(socket: Socket, peer: String, mode: String): Socket =
        if (mode == "setup") socket else client(peer).connect(socket)

    private fun rawExchange(socket: Socket) {
        socket.getOutputStream().write("fixture".toByteArray()); socket.getOutputStream().flush()
        val reply = ByteArray(8)
        java.io.DataInputStream(socket.getInputStream()).readFully(reply)
        assertArrayEquals("accepted".toByteArray(), reply)
    }

    private fun pin(bytes: ByteArray): ConnectionPeer {
        val digest = java.security.MessageDigest.getInstance("SHA-256").digest(bytes)
        return ConnectionPeer.parse(digest.joinToString("") { "%02x".format(it.toInt() and 255) })
    }

    private fun admitted(trust: ConnectionTrust, expected: ConnectionPairing.Phase, remember: Boolean) {
        val pairing = ConnectionPairing()
        val ticket = trust.begin(pairing, pin(fixture("host.der")), 0, 100)
        assertEquals(expected, pairing.phase)
        if (expected == ConnectionPairing.Phase.PENDING) pairing.approve(ticket, remember, 1)
        var lease: ConnectionApproval? = null
        assertTrue(trial { socket ->
            val certificate = (socket as javax.net.ssl.SSLSocket).session.peerCertificates.first().encoded
            lease = trust.authenticated(pairing, ticket, pin(certificate), 2)
        })
        assertTrue(pairing.current(checkNotNull(lease)))
        // trial returns only after both stock TLS sockets and the Rust process retire.
        pairing.cancel(ticket); pairing.retired(ticket)
    }

    @Test @Config(sdk = [27, 34]) fun t852RealTlsRememberRestartRevokeAndUnprovenRejection() {
        val root = Files.createTempDirectory("blent-t852-trust-",
            java.nio.file.attribute.PosixFilePermissions.asFileAttribute(
                java.nio.file.attribute.PosixFilePermissions.fromString("rwx------")))
        val local = pin(fixture("tablet.der")); val peer = pin(fixture("host.der"))
        try {
            ConnectionTrust(AndroidPeerTrustStore.open(root), local).use { trust ->
                rejectedTlsDoesNotPersist(trust)
                admitted(trust, ConnectionPairing.Phase.PENDING, true)
                assertTrue(trust.trusted(peer))
            }
            ConnectionTrust(AndroidPeerTrustStore.open(root), local).use { trust ->
                admitted(trust, ConnectionPairing.Phase.APPROVED, false)
                assertTrue(trust.revoke(ConnectionPairing(), peer))
            }
            ConnectionTrust(AndroidPeerTrustStore.open(root), local).use { trust ->
                assertFalse(trust.trusted(peer))
                admitted(trust, ConnectionPairing.Phase.PENDING, false)
                assertFalse(trust.trusted(peer))
            }
        } finally { root.toFile().deleteRecursively() }
    }

    private fun rejectedTlsDoesNotPersist(trust: ConnectionTrust) {
        for ((peerName, mode) in listOf("stranger" to "accept", "host" to "wrong-preface")) {
            val pairing = ConnectionPairing(); val peer = pin(fixture("host.der"))
            val ticket = trust.begin(pairing, peer, 0, 100)
            pairing.approve(ticket, true, 1)
            assertFalse(trial(peerName, mode) { fail("T852 unverified stream reached persisted admission") })
            assertFalse(trust.trusted(peer))
            assertEquals(ConnectionPairing.Phase.APPROVED, pairing.phase)
            pairing.cancel(ticket); pairing.retired(ticket)
        }
    }

    @Test fun t846Api34RustAndroidMutualTlsAndRejection() {
        assertTrue(trial())
        assertFalse(trial(peer = "stranger"))
        assertFalse(trial(mode = "wrong-preface"))
    }

    @Test @Config(sdk = [27]) fun t846Api27RustAndroidPrefaceWithoutAlpn() {
        assertTrue(trial())
        assertFalse(trial(peer = "stranger"))
        assertFalse(trial(mode = "wrong-preface"))
    }

    private fun window(socket: Socket) {
        val origin = System.nanoTime()
        fun now() = TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - origin)
        val negotiation = ConnectionWindowNegotiation(4096, 0, 3000)
        val input = java.io.DataInputStream(socket.getInputStream())
        val byte = ByteArray(1)
        repeat(ConnectionWindow.BYTES) {
            input.readFully(byte)
            assertEquals(1, negotiation.feed(byte, 0, 1, now()))
        }
        assertNull(negotiation.selected())
        val first = negotiation.output(now())!!
        assertNull(negotiation.output(now()))
        socket.getOutputStream().write(first.bytes, first.offset, 5)
        negotiation.wrote(first, 5, now())
        val rest = negotiation.output(now())!!
        socket.getOutputStream().write(rest.bytes, rest.offset, rest.length)
        negotiation.wrote(rest, rest.length, now())
        assertNull(negotiation.selected())
        val receipt = negotiation.flush(now())!!
        socket.getOutputStream().flush()
        assertEquals(4096, negotiation.flushed(receipt, now()))
        assertEquals(4096, negotiation.selected())
        assertFalse(negotiation.nativePending())
    }

    private fun framed(socket: Socket) {
        val origin = System.nanoTime()
        val clock = { TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - origin) }
        val mux = ConnectionMux(1, 1, 4096, 8, 64)
        val reader = ConnectionFrameReader(3000)
        val writer = ConnectionFrameWriter(3000)
        assertTrue(mux.offer(ConnectionWire.MANAGEMENT, "window-frame".toByteArray()))
        repeat(16) {
            drainFrames(socket, mux, writer, clock)
            val frame = readFrame(socket, reader, clock)
            assertEquals(ConnectionWire.MANAGEMENT, frame.header.channel)
            val delivered = mux.receive(frame.header)
            if (delivered) {
                assertArrayEquals("framed-ok".toByteArray(), frame.bytes.copyOfRange(frame.offset, frame.offset + frame.length))
                mux.release(frame.header.channel, frame.length)
            }
            reader.release(frame)
            if (delivered) return
        }
        error("Rust framing turn limit exceeded")
    }

    private fun drainFrames(socket: Socket, mux: ConnectionMux, writer: ConnectionFrameWriter, now: () -> Long) {
        repeat(8) {
            val frame = mux.next() ?: return
            val owned = ConnectionFrame(frame.header, frame.bytes, frame.offset, frame.length)
            writer.begin(owned, now())
            writeFrame(socket, writer, now)
            val receipt = writer.flush(now())!!
            assertNull(writer.flush(now()))
            socket.getOutputStream().flush()
            assertSame(owned, writer.flushComplete(receipt, now()))
            mux.finish(true)
        }
        error("Unbounded fixture output")
    }

    private fun writeFrame(socket: Socket, writer: ConnectionFrameWriter, now: () -> Long) {
        repeat(128) {
            val loan = writer.write(now()) ?: return
            assertNull(writer.write(now()))
            val count = minOf(7, loan.length)
            socket.getOutputStream().write(loan.bytes, loan.offset, count)
            writer.writeComplete(loan, count, now())
        }
        error("Unbounded fixture frame")
    }

    private fun readFrame(socket: Socket, reader: ConnectionFrameReader, now: () -> Long): ConnectionFrame {
        repeat(128) {
            reader.frame()?.let { frame -> assertNull(reader.frame()); return frame }
            val loan = reader.read(now())!!
            assertNull(reader.read(now()))
            val count = socket.getInputStream().read(loan.bytes, loan.offset, minOf(7, loan.length))
            reader.readComplete(loan, count, now())
        }
        error("Unbounded fixture input")
    }

    @Test @Config(sdk = [27, 34]) fun t881RealTlsWindowNegotiationMatchesHostRuntimeBeforeApplicationData() {
        assertTrue(trial(mode = "window") { socket -> window(socket); framed(socket) })
    }

    private fun authenticatedSession(socket: Socket): TabletSession {
        val certificate = (socket as javax.net.ssl.SSLSocket).session.peerCertificates.first().encoded
        val pairing = ConnectionPairing()
        val ticket = pairing.begin(pin(certificate), true, 0, 3000)
        val authority = pairing.authenticated(ticket, pin(certificate), 0)
        assertTrue(pairing.current(authority))
        return TabletSession(ticket, ConnectionSessionLimits(4096, 64, 64, 3000), 15)
    }

    private fun resourceSession(socket: Socket) {
        window(socket)
        val session = authenticatedSession(socket)
        val reader = ConnectionFrameReader(3000)
        val writer = ConnectionFrameWriter(3000)
        val origin = System.nanoTime()
        val clock = { TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - origin) }
        var retired = 0
        repeat(256) {
            drainSession(socket, session, writer, clock)
            val events = resourceEvents(session, clock())
            retired += events.second
            if (retired == 2) return
            if (events.first) return@repeat
            val frame = readFrame(socket, reader, clock)
            session.receive(frame, clock()); reader.release(frame)
        }
        error("T882 resource session turn limit exceeded")
    }

    private fun drainSession(socket: Socket, session: ConnectionSession, writer: ConnectionFrameWriter, now: () -> Long) {
        repeat(32) {
            val frame = session.nextFrame(now()) ?: return
            writer.begin(frame, now()); writeFrame(socket, writer, now)
            val receipt = checkNotNull(writer.flush(now()))
            socket.getOutputStream().flush()
            assertSame(frame, writer.flushComplete(receipt, now()))
            session.written(frame, now())
        }
        error("T882 unbounded resource output")
    }

    private fun resourceEvents(session: ConnectionEndpoints, now: Long, observed: (ConnectionResourceLease) -> Unit = {}): Pair<Boolean, Int> {
        var progress = false
        var retired = 0
        repeat(32) {
            val event = session.nextEvent() ?: return progress to retired
            progress = true
            when (event) {
                is ConnectionSessionEvent.Offered -> {
                    assertEquals(ConnectionManagement.Resource.DISPLAY, event.lease.key.resource)
                    assertTrue(event.lease.key.generation in 1..2)
                    assertArrayEquals(ByteArray(32) { 7 }, event.credential.bytes())
                    observed(event.lease)
                    session.prepared(event.lease, now)
                }
                is ConnectionSessionEvent.Readable -> resourceEcho(session, event)
                is ConnectionSessionEvent.State -> retired += resourceState(session, event)
            }
        }
        error("T882 unbounded resource events")
    }

    private fun resourceState(session: ConnectionEndpoints, event: ConnectionSessionEvent.State): Int {
        when (event.state) {
            ConnectionResourceState.ADMITTED -> assertTrue(session.current(event.lease))
            ConnectionResourceState.CLOSING -> session.nativeRetired(event.lease)
            ConnectionResourceState.RETIRED -> { assertFalse(session.current(event.lease)); return 1 }
        }
        return 0
    }

    private fun resourceEcho(session: ConnectionEndpoints, event: ConnectionSessionEvent.Readable) {
        val read = checkNotNull(session.read(event.lease, event.channel, 3))
        val bytes = read.bytes.copyOfRange(read.offset, read.offset + read.length)
        assertArrayEquals(byteArrayOf(event.lease.key.generation.toByte(), event.channel.toByte(), 42), bytes)
        session.consumed(event.lease, event.channel, read)
        assertTrue(session.offer(event.lease, event.channel, bytes))
    }

    @Test @Config(sdk = [27, 34]) fun t882RealTlsResourceAdmissionDuplexRetirementAndReplacement() {
        assertTrue(trial(mode = "session", verified = ::resourceSession))
    }

    private fun nativeResourceSession(socket: Socket) {
        window(socket)
        val session = authenticatedSession(socket)
        val wakes = java.util.concurrent.ArrayBlockingQueue<Unit>(1)
        val io = StreamConnectionIo(socket.getInputStream(), socket.getOutputStream(),
            closer = java.io.Closeable { socket.close() }, ready = { wakes.offer(Unit); Unit })
        val runner = ConnectionSessionRunner(session, io, ConnectionRunLimits(3000, 1000))
        val origin = System.nanoTime()
        val clock = { TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - origin) }
        try { driveNativeSession(runner, wakes, clock) }
        finally { retireNativeSession(runner, wakes, clock) }
        assertTrue(io.retired()); assertNull(io.closeFailure()); assertTrue(socket.isClosed)
    }

    private fun driveNativeSession(runner: ConnectionRun, wakes: java.util.concurrent.ArrayBlockingQueue<Unit>, now: () -> Long,
        authority: () -> Boolean = { true }) {
        var retired = 0
        val deadline = now() + 5000
        while (now() < deadline) {
            check(authority()) { "Setup authority was revoked before dispatch" }
            runner.advance(now())
            val events = resourceEvents(runner.endpoints(), now())
            retired += events.second
            if (retired == 2) return
            check(!runner.retired()) { "T883 premature native retirement: ${runner.failure()}" }
            // Local endpoint commands may queue output without a native wake.
            if (!events.first) wakes.poll(10, TimeUnit.MILLISECONDS)
        }
        error("T883 native resource exchange deadline: ${runner.failure()}")
    }

    private fun retireNativeSession(runner: ConnectionRun, wakes: java.util.concurrent.ArrayBlockingQueue<Unit>, now: () -> Long) {
        runner.stop(now())
        val deadline = now() + 2000
        while (!runner.retired()) {
            check(now() < deadline) { "T883 actual native retirement missing: ${runner.failure()}" }
            runner.advance(now())
            wakes.poll(10, TimeUnit.MILLISECONDS)
        }
        assertFalse(runner.terminationRequired())
    }

    @Test @Config(sdk = [27, 34]) fun t883RealTlsNativeRunnerDuplexGenerationsAndActualRetirement() {
        assertTrue(trial(mode = "session", verified = ::nativeResourceSession))
    }

    private fun setupIdentity(): ClientTlsIdentity {
        val bytes = fixture("tablet.der")
        val certificate = CertificateFactory.getInstance("X.509").generateCertificate(bytes.inputStream()) as java.security.cert.X509Certificate
        val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(fixture("tablet.pk8")))
        return ClientTlsIdentity(ConnectionIdentity(bytes), IdentityKeyManager("tablet", key, certificate))
    }

    private fun setupEvents(owner: ConnectionSetup, expectPrompt: Boolean): Boolean {
        var prompted = false
        repeat(32) {
            when (val event = owner.poll() ?: return prompted) {
                is ConnectionWorkerEvent.Pairing -> {
                    assertTrue(expectPrompt)
                    assertEquals(pin(fixture("host.der")), event.ticket.peer)
                    assertEquals(pin(fixture("tablet.der")), event.local)
                    assertTrue(owner.decide(event.ticket, true, true)); prompted = true
                }
                is ConnectionWorkerEvent.Ended -> error("Setup ended before admission: ${owner.failure()}")
                else -> Unit
            }
        }
        error("Unbounded setup events")
    }

    private fun setupReady(owner: ConnectionSetup, wakes: java.util.concurrent.ArrayBlockingQueue<Unit>,
        clock: () -> Long, expectPrompt: Boolean): ConnectionSetupReady {
        val deadline = clock() + 5000
        var prompted = false
        while (clock() < deadline) {
            owner.advance()
            prompted = setupEvents(owner, expectPrompt) || prompted
            owner.takeReady()?.let { assertEquals(expectPrompt, prompted); return it }
            check(owner.failure() == null) { "Native setup failed: ${owner.failure()}" }
            wakes.poll(10, TimeUnit.MILLISECONDS)
        }
        error("Setup did not publish authenticated runtime")
    }

    private fun setupSession(socket: Socket, directory: java.nio.file.Path, expectPrompt: Boolean) {
        val wakes = java.util.concurrent.ArrayBlockingQueue<Unit>(1)
        val origin = System.nanoTime()
        val clock = { TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - origin) }
        val backend = SocketConnectionSetup(socket, ::setupIdentity)
        val owner = ConnectionSetupOwner(backend, { AndroidPeerTrustStore.open(directory) }, clock,
            ConnectionSetupLimits(4096, 5000, 3000, 2000), { wakes.offer(Unit); Unit })
        try {
            val ready = setupReady(owner, wakes, clock, expectPrompt)
            assertTrue(owner.current(ready.approval)); assertEquals(4096, ready.capacity)
            val session = TabletSession(ready.approval.ticket, ConnectionSessionLimits(ready.capacity, 64, 64, 3000), 15)
            val runner = ConnectionSessionRunner(session, ready.io, ConnectionRunLimits(3000, 1000))
            try { driveNativeSession(runner, wakes, clock) { owner.current(ready.approval) } }
            finally { retireNativeSession(runner, wakes, clock) }
            if (!expectPrompt) {
                assertTrue(owner.revoke(ready.approval.ticket.peer)); assertFalse(owner.current(ready.approval))
            }
        } finally { retireSetup(owner, wakes, clock) }
        assertTrue(socket.isClosed)
    }

    private fun retireSetup(owner: ConnectionSetup, wakes: java.util.concurrent.ArrayBlockingQueue<Unit>, clock: () -> Long) {
        owner.stop()
        val deadline = clock() + 3000
        while (!owner.retired()) {
            check(clock() < deadline) { "Setup native ownership did not retire: ${owner.failure()}" }
            owner.advance(); wakes.poll(10, TimeUnit.MILLISECONDS)
        }
        assertFalse(owner.terminationRequired())
    }

    @Test @Config(sdk = [27, 34]) fun t886RealSetupDurableRestartDuplexAndRevoke() {
        val directory = Files.createTempDirectory("blent-t886-trust-",
            java.nio.file.attribute.PosixFilePermissions.asFileAttribute(
                java.nio.file.attribute.PosixFilePermissions.fromString("rwx------")))
        try {
            assertTrue(trial(mode = "setup") { setupSession(it, directory, true) })
            assertTrue(trial(mode = "setup") { setupSession(it, directory, false) })
            ConnectionTrust(AndroidPeerTrustStore.open(directory), pin(fixture("tablet.der"))).use {
                assertFalse(it.trusted(pin(fixture("host.der"))))
            }
        } finally { directory.toFile().deleteRecursively() }
    }


    private fun runtimePairing(runtime: ConnectionRuntime, expectPrompt: Boolean): Boolean {
        var prompted = false
        repeat(32) {
            when (val event = runtime.poll() ?: return prompted) {
                is ConnectionWorkerEvent.Pairing -> {
                    assertTrue(expectPrompt)
                    assertEquals(pin(fixture("host.der")), event.ticket.peer)
                    assertEquals(pin(fixture("tablet.der")), event.local)
                    assertTrue(runtime.command(ConnectionWorkerCommand.Decide(event.ticket, true, true)))
                    prompted = true
                }
                is ConnectionWorkerEvent.Ended -> error("Runtime ended before resource exchange: ${runtime.failure()}")
                else -> Unit
            }
        }
        error("Unbounded runtime events")
    }

    private fun driveRuntime(runtime: ConnectionRuntime, wakes: java.util.concurrent.ArrayBlockingQueue<Unit>,
        clock: () -> Long, expectPrompt: Boolean): ConnectionResourceLease {
        var prompted = false
        var retired = 0
        var last: ConnectionResourceLease? = null
        val deadline = clock() + 5000
        while (clock() < deadline) {
            runtime.advance()
            val events = resourceEvents(runtime.endpoints(), clock()) { last = it }
            retired += events.second
            if (retired == 2) {
                assertEquals(expectPrompt, prompted)
                return checkNotNull(last)
            }
            prompted = runtimePairing(runtime, expectPrompt) || prompted
            check(!runtime.retired()) { "Runtime retired before duplex exchange: ${runtime.failure()}" }
            if (!events.first) wakes.poll(10, TimeUnit.MILLISECONDS)
        }
        error("Runtime duplex deadline: ${runtime.failure()}")
    }

    private fun retireRuntime(runtime: ConnectionRuntime, wakes: java.util.concurrent.ArrayBlockingQueue<Unit>, clock: () -> Long) {
        runtime.stop()
        val deadline = clock() + 3000
        while (!runtime.retired()) {
            check(clock() < deadline) { "Runtime native ownership did not retire: ${runtime.failure()}" }
            runtime.advance(); wakes.poll(10, TimeUnit.MILLISECONDS)
        }
        assertFalse(runtime.terminationRequired())
    }

    private fun runtimeSession(socket: Socket, directory: java.nio.file.Path, expectPrompt: Boolean) {
        val wakes = java.util.concurrent.ArrayBlockingQueue<Unit>(1)
        val origin = System.nanoTime()
        val clock = { TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - origin) }
        val setup = ConnectionSetupOwner(SocketConnectionSetup(socket, ::setupIdentity),
            { AndroidPeerTrustStore.open(directory) }, clock,
            ConnectionSetupLimits(4096, 5000, 3000, 2000), { wakes.offer(Unit); Unit })
        val runtime = ConnectionRuntimeOwner(setup, ConnectionRunFactory { ready ->
            assertEquals(4096, ready.capacity)
            ConnectionSessionRunner(TabletSession(ready.approval.ticket,
                ConnectionSessionLimits(ready.capacity, 64, 64, 3000), 15), ready.io, ConnectionRunLimits(3000, 1000))
        }, clock)
        try {
            val lease = driveRuntime(runtime, wakes, clock, expectPrompt)
            if (!expectPrompt) assertTrue(runtime.command(ConnectionWorkerCommand.Revoke(lease.connection.peer)))
            runtime.stop()
            assertFalse(runtime.endpoints().current(lease))
            assertFalse(runtime.endpoints().offer(lease, ConnectionWire.MANAGEMENT, byteArrayOf(1)))
        } finally { retireRuntime(runtime, wakes, clock) }
        assertTrue(socket.isClosed)
    }

    @Test @Config(sdk = [27, 34]) fun t888RealRuntimeSetupDuplexRestartRevokeAndActualRetirement() {
        val directory = Files.createTempDirectory("blent-t888-trust-",
            java.nio.file.attribute.PosixFilePermissions.asFileAttribute(
                java.nio.file.attribute.PosixFilePermissions.fromString("rwx------")))
        try {
            assertTrue(trial(mode = "setup") { runtimeSession(it, directory, true) })
            assertTrue(trial(mode = "setup") { runtimeSession(it, directory, false) })
            ConnectionTrust(AndroidPeerTrustStore.open(directory), pin(fixture("tablet.der"))).use {
                assertFalse(it.trusted(pin(fixture("host.der"))))
            }
        } finally { directory.toFile().deleteRecursively() }
    }

}
