package com.blent.connection

/** T883 bounded reactor: no native operation runs on its serialized owner.
 * One native request per direction, with exact completions and fixed buffers.
 * Native/process supervision is supplied by separate adapters. */
internal class ConnectionSessionRunner(
    private val session: ConnectionSession,
    private val io: ConnectionIo,
    private val limits: ConnectionRunLimits = ConnectionRunLimits(),
) : ConnectionRun, ConnectionDeadlineSource {
    private val reader = ConnectionFrameReader(limits.ioTimeoutMs)
    private val writer = ConnectionFrameWriter(limits.ioTimeoutMs)
    private var reading: ConnectionIoRequest.Read? = null
    private var writing: ConnectionIoRequest? = null
    private var frame: ConnectionFrame? = null
    private var previous = 0L
    private var stoppedAt: Long? = null
    private var ended = false
    private var terminate = false
    private var error: Exception? = null

    private fun time(now: Long) {
        require(now >= previous && now <= Long.MAX_VALUE - limits.retirementTimeoutMs)
        previous = now
    }
    override fun advance(now: Long) {
        time(now)
        if (ended) return
        try {
            if (stoppedAt == null) active(now) else completions(now)
        } catch (failure: Exception) {
            if (error == null) error = failure
            stop(now)
        }
        retirement(now)
    }
    private fun active(now: Long) {
        reader.tick(now); writer.tick(now); session.tick(now)
        completions(now)
        startRead(now)
        startWrite(now)
    }
    private fun completions(now: Long) {
        // Only two native directions exist; never spin on an untrusted adapter.
        repeat(2) {
            val completion = io.poll() ?: return
            when (val request = completion.request) {
                is ConnectionIoRequest.Read -> readComplete(request, completion, now)
                else -> writeComplete(request, completion, now)
            }
        }
    }
    private fun readComplete(request: ConnectionIoRequest.Read, result: ConnectionIoCompletion, now: Long) {
        require(reading === request) { "Foreign or retired native read completion" }
        reading = null
        if (stoppedAt != null) return
        result.failure?.let { throw it }
        reader.readComplete(request.loan, result.count, now)
        val complete = reader.frame() ?: return
        try { session.receive(complete, now) } finally { reader.release(complete) }
    }
    private fun writeComplete(request: ConnectionIoRequest, result: ConnectionIoCompletion, now: Long) {
        require(writing === request) { "Foreign or retired native write completion" }
        writing = null
        if (stoppedAt != null) return
        result.failure?.let { throw it }
        when (request) {
            is ConnectionIoRequest.Write -> writer.writeComplete(request.loan, result.count, now)
            is ConnectionIoRequest.Flush -> {
                require(result.count == 0)
                val complete = writer.flushComplete(request.loan, now)
                frame = null
                session.written(complete, now)
            }
            else -> error("Read completion reached writer")
        }
    }
    private fun startRead(now: Long) {
        if (reading != null) return
        val loan = reader.read(now) ?: return
        val request = ConnectionIoRequest.Read(loan)
        reading = request
        check(io.submit(request)) { "Native read capacity lost" }
    }
    private fun startWrite(now: Long) {
        if (writing != null) return
        if (frame == null) {
            frame = session.nextFrame(now) ?: return
            writer.begin(checkNotNull(frame), now)
        }
        val request = nextWrite(now) ?: return
        writing = request
        check(io.submit(request)) { "Native write capacity lost" }
    }
    private fun nextWrite(now: Long): ConnectionIoRequest? {
        writer.write(now)?.let { return ConnectionIoRequest.Write(it) }
        return writer.flush(now)?.let { ConnectionIoRequest.Flush(it) }
    }
    override fun stop(now: Long) {
        time(now)
        if (stoppedAt != null) return
        stoppedAt = now
        session.disconnect(); reader.cancel(); writer.cancel()
        io.stop()
    }
    private fun retirement(now: Long) {
        val started = stoppedAt ?: return
        if (ended) return
        if (io.retired()) {
            io.closeFailure()?.let { if (error == null) error = it; terminate = true }
            session.transportRetired()
            ended = true
            reading = null; writing = null; frame = null
        } else if (now - started >= limits.retirementTimeoutMs) terminate = true
    }
    override fun display(): LocalDisplayRoute? = null
    override fun endpoints(): ConnectionEndpoints = session
    override fun failure(): Exception? = error
    override fun retired(): Boolean = ended
    override fun terminationRequired(): Boolean = terminate
    override fun nextDeadline(): ConnectionDeadline {
        if (ended || terminate) return ConnectionDeadline.IDLE
        val stopped = stoppedAt
        if (stopped != null) return ConnectionDeadline.at(stopped + limits.retirementTimeoutMs)
        return reader.nextDeadline().combine(writer.nextDeadline())
            .combine(ConnectionDeadline.observe(session as? ConnectionDeadlineSource))
    }
}
