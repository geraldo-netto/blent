package com.blent.connection

import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*

/** One exact native thread/call and one original coroutine. Every exit releases
 * the fixture's block, joins the original coroutine, then joins its thread. */
internal class PreferenceStorageRig(private val blockWrite: Boolean) : AutoCloseable {
    val entered = CountDownLatch(1)
    private val release = CountDownLatch(1)
    val reads = AtomicInteger()
    val writes = AtomicInteger()
    val encoded = AtomicReference<String>()
    private val thread = AtomicReference<Thread>()
    private val executor = Executors.newSingleThreadExecutor { action ->
        Thread(action, "blent-preference-storage-fixture").also { check(thread.compareAndSet(null, it)) }
    }
    private val dispatcher = executor.asCoroutineDispatcher()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val storage = object : ConnectionPreferenceStorage {
        override fun read(): String? {
            reads.incrementAndGet()
            if (!blockWrite) block()
            return encoded.get()
        }
        override fun write(encoded: String): Boolean {
            writes.incrementAndGet()
            this@PreferenceStorageRig.encoded.set(encoded)
            if (blockWrite) block()
            return true
        }
    }
    val store = ConnectionPreferenceStore(storage, dispatcher)
    private var pending: Deferred<Result<ConnectionWorkerSpec>>? = null
    fun begin(): Deferred<Result<ConnectionWorkerSpec>> {
        check(pending == null)
        return scope.async(start = CoroutineStart.UNDISPATCHED) {
            if (blockWrite) store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb", timerMs = 1000)))
            else store.load()
        }.also { pending = it }
    }
    private fun block() { entered.countDown(); release.await() }
    fun awaitAdmission() {
        while (!entered.await(1, TimeUnit.SECONDS)) {
            check(!checkNotNull(pending).isCompleted) { "Original storage call ended without admission" }
            println("T1099 waiting for original fixture storage admission")
        }
    }
    fun settle() { release.countDown() }
    override fun close() {
        settle()
        runBlocking { pending?.cancelAndJoin() }
        scope.cancel()
        dispatcher.close()
        thread.get()?.join()
    }
}
