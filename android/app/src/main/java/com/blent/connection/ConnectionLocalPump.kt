package com.blent.connection

/** One local media channel. A serialized owner calls advance on an independent
 * timer and on wakeups. Native I/O belongs to the supplied prompt ConnectionIo;
 * this pump never opens sockets, blocks, cancels siblings or releases a resource
 * lease. Its resource owner calls nativeRetired only after ALL channel pumps and
 * listener/native owners truly retire. A stuck close quarantines this resource.
 * One bounded block per direction; incoming ring bytes are borrowed without copy
 * until the exact local write AND flush complete. */
internal class ConnectionLocalPump(
    private val endpoints: ConnectionEndpoints,
    private val lease: ConnectionResourceLease,
    private val channel: Int,
    private val io: ConnectionIo,
    blockBytes: Int = ConnectionWire.MAX_PAYLOAD,
    private val limits: ConnectionRunLimits = ConnectionRunLimits(),
) {
    private val producer = LocalChannelProducer(blockBytes, limits.ioTimeoutMs)
    private val consumer = LocalChannelConsumer(blockBytes, limits.ioTimeoutMs)
    private var previous = 0L
    private var stoppedAt: Long? = null
    private var finished = false
    private var closed = false
    private var error: Exception? = null
    init {
        require(ConnectionChannels.belongs(lease.key.resource, channel))
        require(lease.owner > 0 && lease.connection.epoch > 0)
        require(lease.key.request > 0 && lease.key.generation > 0)
    }
    private fun time(now: Long) {
        require(now >= previous && now <= Long.MAX_VALUE - limits.retirementTimeoutMs)
        previous = now
    }
    fun advance(now: Long) {
        time(now)
        if (finished) return
        try {
            admission(now)
            completions(now)
            if (stoppedAt == null) {
                producer.advance(endpoints, lease, channel, io, now)
                consumer.advance(endpoints, lease, channel, io, now)
            }
        } catch (failure: Exception) { fail(failure); stop(now) }
        retirement()
    }
    private fun admission(now: Long) {
        if (stoppedAt != null) return
        if (!endpoints.current(lease)) { stop(now); return }
        producer.tick(now)
        consumer.tick(now)
    }
    private fun completions(now: Long) {
        // One request per direction; faulty adapters cannot create an unbounded loop.
        repeat(2) {
            val completion = io.poll() ?: return
            when (completion.request) {
                is ConnectionIoRequest.Read -> producer.complete(completion, stoppedAt != null, now)
                else -> consumer.complete(completion, endpoints, lease, channel, stoppedAt != null, now)
            }
        }
    }
    fun stop(now: Long) {
        time(now)
        if (stoppedAt != null) return
        stoppedAt = now
        try { io.stop() } catch (failure: Exception) { fail(failure) }
    }
    private fun fail(failure: Exception) { if (error == null) error = failure }
    private fun retirement() {
        if (stoppedAt == null || !io.retired()) return
        val failure = io.closeFailure()
        if (failure != null) fail(failure)
        closed = failure == null
        finished = true
    }
    fun failure(): Exception? = error
    fun retired(): Boolean = finished && closed
    fun quarantined(): Boolean = stoppedAt?.let { !retired() && (finished || previous - it >= limits.retirementTimeoutMs) } == true
}

/** Local producer retains one block until the authenticated channel accepts it.
 * An idle native read has no data-progress deadline. Backpressure after bytes
 * arrive does; further wakeups never reset that absolute budget. */
internal class LocalChannelProducer(private val block: Int, timeoutMs: Long) {
    private val clock = ConnectionFrameClock(timeoutMs)
    private var native: ConnectionIoRequest.Read? = null
    private var pending: ByteArray? = null
    private var reusable: ByteArray? = null
    init { require(block in 1..ConnectionWire.MAX_PAYLOAD) }

    fun tick(now: Long) = clock.check(now)

    fun advance(endpoint: ConnectionEndpoints, lease: ConnectionResourceLease, channel: Int, io: ConnectionIo, now: Long) {
        clock.check(now)
        pending?.let {
            if (!endpoint.offer(lease, channel, it)) return
            pending = null
            clock.clear()
        }
        if (native != null) return
        val bytes = reusable ?: ByteArray(block)
        reusable = null
        val request = ConnectionIoRequest.Read(ConnectionBuffer(bytes, 0, block))
        native = request
        check(io.submit(request)) { "Local read admission lost" }
    }
    fun complete(result: ConnectionIoCompletion, stopped: Boolean, now: Long) {
        val request = checkNotNull(native)
        require(result.request === request) { "Foreign or retired local read" }
        native = null
        if (stopped) return
        result.failure?.let { throw it }
        require(result.count in 0..request.loan.length) { "Local producer EOF or invalid read extent" }
        // Partial payloads own a copy; only their completed scratch can return.
        // Full blocks transfer permanently through pending and endpoint.offer.
        reusable = if (result.count < block) request.loan.bytes else null
        if (result.count == 0) return
        clock.start(now)
        pending = if (result.count == block) request.loan.bytes else request.loan.bytes.copyOf(result.count)
    }
}

/** Serialized local consumer keeps the ring receipt while partial writes and
 * flush are pending. No credit or array reuse on cancellation, failure or timeout. */
internal class LocalChannelConsumer(private val block: Int, timeoutMs: Long) {
    private val clock = ConnectionFrameClock(timeoutMs)
    private var receipt: ConnectionChannelRead? = null
    private var offset = 0
    private var native: ConnectionIoRequest? = null
    init { require(block in 1..ConnectionWire.MAX_PAYLOAD) }

    fun tick(now: Long) = clock.check(now)

    fun advance(endpoint: ConnectionEndpoints, lease: ConnectionResourceLease, channel: Int, io: ConnectionIo, now: Long) {
        clock.check(now)
        if (native != null) return
        if (receipt == null) {
            val read = endpoint.read(lease, channel, block) ?: return
            require(read.offset in 0..read.bytes.size && read.length in 1..block)
            require(read.length <= read.bytes.size - read.offset)
            receipt = read
            offset = 0
            clock.start(now)
        }
        val read = checkNotNull(receipt)
        val request = if (offset < read.length) ConnectionIoRequest.Write(ConnectionBuffer(read.bytes, read.offset + offset, read.length - offset))
            else ConnectionIoRequest.Flush(ConnectionFlush())
        native = request
        check(io.submit(request)) { "Local write admission lost" }
    }
    fun complete(result: ConnectionIoCompletion, endpoint: ConnectionEndpoints, lease: ConnectionResourceLease, channel: Int, stopped: Boolean, now: Long) {
        require(native != null && result.request === native) { "Foreign or retired local write" }
        native = null
        if (stopped) return
        result.failure?.let { throw it }
        clock.check(now)
        when (val request = result.request) {
            is ConnectionIoRequest.Write -> {
                require(result.count in 1..request.loan.length) { "Invalid local write progress" }
                offset += result.count
            }
            is ConnectionIoRequest.Flush -> {
                require(result.count == 0)
                endpoint.consumed(lease, channel, checkNotNull(receipt))
                receipt = null
                clock.clear()
            }
            else -> error("Local read delivered to consumer")
        }
    }
}
