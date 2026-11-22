package com.blent.connection

import java.io.File
import java.io.Closeable
import java.net.InetSocketAddress
import java.net.Socket
import java.nio.file.Files
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket

/** Original Process and socket owners remain here through all constructor/use
 * failures. Only this private helper is cancelled; installed sessions are absent. */
internal class RustReadConnection(helper: File, fixtures: File, client: SSLContext,
                                  size: Int, frames: Int, pipeline: Int) : MeasurementConnection {
    private val directory = Files.createTempDirectory("blent-t1770-").toFile()
    private val ready = File(directory, "ready")
    private val log = File(directory, "peer.log")
    private val resources = ArrayList<Closeable>(3)
    private var original: Process? = null
    private var finished = false
    private lateinit var socket: SSLSocket
    private lateinit var metered: MeteredSocket
    val alive: Boolean get() = original?.isAlive ?: false
    val closed: Boolean get() = resources.filterIsInstance<Socket>().all { it.isClosed }
    val records: Long get() = metered.incoming.records
    override val input: java.io.InputStream get() = socket.inputStream
    override val output: java.io.OutputStream get() = socket.outputStream
    init {
        try {
            require(size in 1..ConnectionWire.MAX_PAYLOAD)
            require(frames in 1..60 && pipeline in 1..minOf(4, frames))
            original = ProcessBuilder(helper.absolutePath, fixtures.absolutePath, ready.absolutePath,
                size.toString(), frames.toString(), pipeline.toString())
                .redirectErrorStream(true).redirectOutput(log).start()
            connect(client, awaitPort())
            readInitialCredits(size * pipeline)
        } catch (failure: Throwable) {
            try { close() } catch (cleanup: Throwable) { failure.addSuppressed(cleanup) }
            throw failure
        }
    }
    private fun awaitPort(): Int {
        val deadline = System.nanoTime() + 5_000_000_000L // Private setup protocol deadline.
        while (true) {
            check(checkNotNull(original).isAlive) { log.readText() }
            if (ready.isFile) ready.readText().trim().toIntOrNull()?.let { return it }
            check(System.nanoTime() < deadline) { "T1770 helper did not publish its endpoint" }
            Thread.sleep(5)
        }
    }
    private fun connect(client: SSLContext, port: Int) {
        val raw = Socket().also { resources.add(it) }
        raw.connect(InetSocketAddress("127.0.0.1", port), 5000)
        raw.soTimeout = 5000; raw.tcpNoDelay = true
        metered = MeteredSocket(raw).also { resources.add(it) }
        socket = (client.socketFactory.createSocket(metered, "blent.local", port, true) as SSLSocket)
            .also { resources.add(it) }
        socket.enabledProtocols = arrayOf("TLSv1.3")
        socket.startHandshake()
        check(socket.session.protocol == "TLSv1.3")
        // Current authenticated wire preface, not a resource/session approval.
        // Host contract is BLENTUSB followed by NUL and version 1 (ten bytes).
        val current = "BLENTUSB".toByteArray(Charsets.US_ASCII) + byteArrayOf(0, 1)
        check(readExact(current.size).contentEquals(current))
        output.write(current); output.flush()
    }
    private fun readExact(count: Int): ByteArray {
        val bytes = ByteArray(count)
        var offset = 0
        while (offset < bytes.size) {
            val read = input.read(bytes, offset, bytes.size - offset)
            check(read > 0) { "T1770 truncated setup" }
            offset += read
        }
        return bytes
    }
    private fun readInitialCredits(capacity: Int) {
        val channels = HashSet<Int>()
        repeat(2) {
            val header = ConnectionWire.decode(readExact(ConnectionWire.HEADER_BYTES))
            check(header == ConnectionWire.Header(ConnectionWire.CREDIT, header.channel, 1, 1, 0, 0, capacity))
            check(channels.add(header.channel))
        }
        check(channels == setOf(ConnectionWire.DISPLAY, ConnectionWire.MANAGEMENT))
    }
    fun finish(io: MeasuredIo) {
        check(io.perform(ConnectionIoRequest.WriteFlushed(ConnectionWriteTransaction(
            ConnectionBuffer(byteArrayOf(127), 0, 1)))) == 1)
        finished = true
    }
    override fun finishPeer() = Unit // Native closer supplies EOF after the framed phase.
    override fun closeStream() = socket.close()
    override fun close() {
        try { closeConnectionResources(resources.reversed()) }
        finally { settleProcess() }
    }
    private fun settleProcess() {
        try {
            original?.let { process ->
                if (!finished) process.destroyForcibly()
                val status = awaitRustProcess(process)
                closeConnectionResources(listOf(process.inputStream, process.errorStream, process.outputStream))
                check(!finished || status == 0) { "T1770 peer failed: ${log.readText()}" }
            }
        } finally {
            ready.delete(); log.delete(); check(directory.delete())
        }
    }
}

/** Cancellation interrupts waiting, not ownership: kill and reap only the exact
 * private helper, then restore the caller interrupt after native settlement. */
internal fun awaitRustProcess(process: Process): Int {
    var interrupted = false
    try {
        while (true) {
            try { return process.waitFor() }
            catch (_: InterruptedException) { interrupted = true; process.destroyForcibly() }
        }
    } finally { if (interrupted) Thread.currentThread().interrupt() }
}
