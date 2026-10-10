package com.blent.connection

import java.io.Closeable
import java.io.InputStream
import java.io.OutputStream
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/** T883 fixed native-worker ownership. Streams and closer transfer on construction.
 * A startup failure remains observable through closeFailure; the returned owner
 * must still be retained until retired() or confirmed containing-process death.
 * The closer must release both stream directions and any underlying native owner.
 * ready is a concurrent, prompt enqueue-only hint, possibly during construction;
 * it must not call the serialized reactor directly. Final worker exit can follow
 * the hint, so an independent supervisor clock must still check retirement. */
internal class StreamConnectionIo(
    private val input: InputStream,
    private val output: OutputStream,
    private val closer: Closeable = Closeable { closeConnectionResources(listOf(input, output)) },
    private val ready: () -> Unit = {},
    private val threadFactory: (String, Runnable) -> Thread = { name, task -> Thread(task, name) },
) : ConnectionIo {
    private val stopped = AtomicBoolean()
    private val closed = AtomicBoolean()
    private val failure = AtomicReference<Exception?>()
    private val inputSlot = StreamIoSlot()
    private val outputSlot = StreamIoSlot()
    private val closeSignal = java.util.concurrent.CountDownLatch(1)
    private val workers = ArrayList<Thread>(3)
    private var cursor = false
    @Volatile private var startupFinished = false

    init {
        try {
            launch("blent-connection-close", Runnable { closeNative() })
            launch("blent-connection-read", Runnable { run(inputSlot) })
            launch("blent-connection-write", Runnable { run(outputSlot) })
        } catch (error: Exception) {
            failure.compareAndSet(null, error)
            stop()
        } finally { startupFinished = true }
    }
    private fun launch(name: String, work: Runnable) {
        val thread = threadFactory(name, work)
        thread.isDaemon = true
        workers.add(thread)
        thread.start()
    }
    override fun submit(request: ConnectionIoRequest): Boolean {
        validate(request)
        if (stopped.get()) return false
        val slot = if (request is ConnectionIoRequest.Read) inputSlot else outputSlot
        return slot.submit(request)
    }
    private fun validate(request: ConnectionIoRequest) {
        val loan = when (request) {
            is ConnectionIoRequest.Read -> request.loan
            is ConnectionIoRequest.Write -> request.loan
            is ConnectionIoRequest.Flush -> return
        }
        require(loan.offset in 0..loan.bytes.size)
        require(loan.length in 1..ConnectionWire.MAX_PAYLOAD && loan.length <= loan.bytes.size - loan.offset)
    }
    @Synchronized override fun poll(): ConnectionIoCompletion? {
        val first = if (cursor) outputSlot else inputSlot
        val second = if (cursor) inputSlot else outputSlot
        val completion = first.poll() ?: second.poll()
        if (completion != null) cursor = !cursor
        return completion
    }
    private fun run(slot: StreamIoSlot) {
        try { drain(slot) }
        catch (error: Exception) {
            failure.compareAndSet(null, error)
            stop()
            signalReady()
        }
    }
    private fun drain(slot: StreamIoSlot) {
        while (true) {
            val request = slot.take() ?: return
            val completion = try {
                check(!stopped.get()) { "Connection I/O stopped before execution" }
                ConnectionIoCompletion(request, perform(request))
            } catch (error: Exception) { ConnectionIoCompletion(request, 0, error) }
            slot.complete(completion)
            signalReady()
        }
    }
    private fun perform(request: ConnectionIoRequest): Int = when (request) {
        is ConnectionIoRequest.Read -> request.loan.let { input.read(it.bytes, it.offset, it.length) }
        is ConnectionIoRequest.Write -> request.loan.let { output.write(it.bytes, it.offset, it.length); it.length }
        is ConnectionIoRequest.Flush -> { output.flush(); 0 }
    }
    override fun stop() {
        if (!stopped.compareAndSet(false, true)) return
        inputSlot.stop(); outputSlot.stop(); closeSignal.countDown()
    }
    private fun closeNative() {
        try { closeSignal.await(); closer.close() }
        catch (error: Exception) { failure.compareAndSet(null, error) }
        finally { closed.set(true); signalReady() }
    }
    private fun signalReady() {
        try { ready() }
        catch (error: Exception) { failure.compareAndSet(null, error); stop() }
    }
    override fun retired(): Boolean = startupFinished && stopped.get() && closed.get() && workers.none { it.isAlive }
    override fun closeFailure(): Exception? = failure.get()
}

/** One mailbox entry, including a completed result not yet consumed by owner. */
private class StreamIoSlot {
    private val lock = java.lang.Object()
    private var request: ConnectionIoRequest? = null
    private var completion: ConnectionIoCompletion? = null
    private var occupied = false
    private var stopped = false
    fun submit(value: ConnectionIoRequest): Boolean = synchronized(lock) {
        if (stopped || occupied) return false
        occupied = true
        request = value
        lock.notifyAll()
        true
    }
    fun take(): ConnectionIoRequest? = synchronized(lock) {
        while (request == null && !stopped) lock.wait()
        request.also { request = null }
    }
    fun complete(value: ConnectionIoCompletion) = synchronized(lock) { completion = value }
    fun poll(): ConnectionIoCompletion? = synchronized(lock) {
        completion?.also { completion = null; occupied = false }
    }
    fun stop() = synchronized(lock) { stopped = true; lock.notifyAll() }
}
