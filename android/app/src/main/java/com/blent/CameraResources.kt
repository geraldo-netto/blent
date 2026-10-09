package com.blent

import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.CancellationException

/** The capture worker owns native destruction; cancellation only interrupts transport. */
internal class CameraResources(private val startupWait: NativeDeadline = NativeDeadline("Camera startup")) {
    val startupStatus get() = startupWait.status
    fun child() = CameraResources(startupWait)
    private val lock = java.lang.Object()
    private var stopping = false
    private var failure: Exception? = null
    private var closeFailure: Exception? = null
    private val uncertain = mutableListOf<() -> Unit>()
    private var closed = false
    private var closing = false
    private var startups = 0
    private val resources = mutableListOf<() -> Unit>()
    private val interrupts = mutableListOf<() -> Unit>()

    fun own(close: () -> Unit) {
        val late = synchronized(lock) {
            if (!closed) resources.add(close)
            closed
        }
        if (late) dispose(close)
        synchronized(lock) {
            if (stopping) throw CancellationException("Camera session stopped")
        }
    }

    /** Only nonblocking transport interruption belongs here, never codec destruction. */
    fun interrupt(close: () -> Unit) {
        synchronized(lock) {
            if (!stopping) { interrupts.add(close); return }
        }
        dispose(close)
        throw CancellationException("Camera session stopped")
    }

    /** Keep retirement waiting for a Camera2 callback even after coroutine cancellation. */
    fun startup(): () -> Unit {
        synchronized(lock) {
            if (stopping) throw CancellationException("Camera session stopped")
            if (startups++ == 0) startupWait.begin()
        }
        val released = AtomicBoolean()
        return {
            if (released.compareAndSet(false, true)) synchronized(lock) {
                if (--startups == 0) startupWait.finish()
                lock.notifyAll()
            }
        }
    }

    fun checkActive() {
        synchronized(lock) {
            failure?.let { throw it }
            if (stopping) throw CancellationException("Camera session stopped")
        }
    }

    fun cancel(error: Exception? = null) {
        val pending = synchronized(lock) {
            if (failure == null) failure = error
            stopping = true
            interrupts.toList().also { interrupts.clear() }
        }
        pending.forEach(::dispose)
    }

    /** Call only after the capture worker has left its borrowed native operations. */
    fun close() {
        cancel()
        val retired = synchronized(lock) {
            while (startups != 0 || closing) lock.wait()
            if (closed) { closeFailure?.let { throw it }; return }
            closed = true
            closing = true
            resources.toList().asReversed().also { resources.clear() }
        }
        try { retired.forEach(::dispose) }
        finally { synchronized(lock) { closing = false; lock.notifyAll() } }
        synchronized(lock) { closeFailure?.let { throw it } }
    }

    private fun dispose(close: () -> Unit) {
        try { close() } catch (error: Exception) {
            synchronized(lock) {
                uncertain.add(close)
                if (closeFailure == null) closeFailure = error
                else if (closeFailure !== error) closeFailure!!.addSuppressed(error)
            }
        }
    }
}
