package com.blent

import kotlinx.coroutines.*
import java.util.concurrent.Executor
import java.util.concurrent.atomic.AtomicBoolean

/** One finite native query, no waiting queue. Cancellation/timeout releases the
 * caller only; ownership lasts until the actual native call returns. */
internal class NativeQueryLane(
    private val deadlineMillis: Long = 5_000,
    private val executor: Executor = Executor { work ->
        Thread(work, "blent-metadata").apply { isDaemon = true }.start()
    },
) {
    init { require(deadlineMillis in 1..60_000) }
    private val occupied = AtomicBoolean()
    val busy: Boolean get() = occupied.get()

    suspend fun <T : Any> query(read: () -> T): T? {
        currentCoroutineContext().ensureActive()
        if (!occupied.compareAndSet(false, true)) return null
        val result = CompletableDeferred<T?>()
        try { executor.execute { execute(read, result) } }
        catch (_: Exception) { occupied.set(false); return null }
        return withTimeoutOrNull(deadlineMillis) { result.await() }
    }

    private fun <T : Any> execute(read: () -> T, result: CompletableDeferred<T?>) {
        val value = try { read() } catch (_: Exception) { null }
            finally { occupied.set(false) }
        result.complete(value)
    }
}

/** Shared by shell inventory and control negotiation, so they cannot create
 * independent replacement workers around the same stalled codec service. */
internal object MetadataQueries { val lane = NativeQueryLane() }
