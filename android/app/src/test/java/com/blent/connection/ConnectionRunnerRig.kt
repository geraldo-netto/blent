package com.blent.connection

import java.util.ArrayDeque

internal class RunnerIo : ConnectionIo {
    val requests = ArrayList<ConnectionIoRequest>()
    val results = ArrayDeque<ConnectionIoCompletion>()
    var accepting = true
    var stopping = false
    var gone = false
    var closeError: Exception? = null
    override fun submit(request: ConnectionIoRequest): Boolean {
        if (!accepting) return false
        requests.add(request)
        return true
    }
    override fun poll(): ConnectionIoCompletion? = results.pollFirst()
    override fun stop() { stopping = true }
    override fun retired() = gone
    override fun closeFailure() = closeError
    fun complete(request: ConnectionIoRequest, count: Int, failure: Exception? = null) {
        results.addLast(ConnectionIoCompletion(request, count, failure))
    }
    fun read(): ConnectionIoRequest.Read = requests.filterIsInstance<ConnectionIoRequest.Read>().last()
    fun write(): ConnectionIoRequest.Write = requests.filterIsInstance<ConnectionIoRequest.Write>().last()
    fun flush(): ConnectionIoRequest.Flush = requests.filterIsInstance<ConnectionIoRequest.Flush>().last()
}

internal class RunnerSession : ConnectionSession by TabletSession(
    ConnectionTicket(1, ConnectionPeer.parse("ab".repeat(32))), ConnectionSessionLimits(), 15,
) {
    val output = ArrayDeque<ConnectionFrame>()
    val input = ArrayList<ConnectionFrame>()
    val written = ArrayList<ConnectionFrame>()
    var disconnected = false
    var retirementCount = 0
    var clock = 0L
    override fun receive(frame: ConnectionFrame, now: Long) {
        input.add(frame.copy(bytes = frame.bytes.copyOfRange(frame.offset, frame.offset + frame.length), offset = 0))
    }
    override fun nextFrame(now: Long): ConnectionFrame? = output.pollFirst()
    override fun written(frame: ConnectionFrame, now: Long) { written.add(frame) }
    override fun tick(now: Long) { clock = now }
    override fun disconnect() { disconnected = true }
    override fun transportRetired() { retirementCount += 1 }
}

internal class ConnectionRunnerRig(timeout: Long = 10) {
    val io = RunnerIo()
    val session = RunnerSession()
    val runner = ConnectionSessionRunner(session, io, ConnectionRunLimits(timeout, 5))
    fun frame(body: ByteArray = byteArrayOf(1, 2, 3)) = ConnectionFrame(
        ConnectionWire.Header(ConnectionWire.DATA, 6, 1, 1, 0, body.size, 0), body, 0, body.size)
    fun encoded(frame: ConnectionFrame): ByteArray {
        val bytes = ByteArray(ConnectionWire.HEADER_BYTES)
        ConnectionWire.encode(frame.header, bytes)
        return bytes + frame.bytes.copyOfRange(frame.offset, frame.offset + frame.length)
    }
}
