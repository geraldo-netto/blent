package com.blent.connection

import kotlinx.coroutines.*
import kotlinx.coroutines.sync.Mutex

/** Synchronous application storage calls over immutable records. The backend
 * owns its internal storage resources; return settles this caller's API borrow.
 * write true confirms publication; false may still expose an unconfirmed value. */
internal interface ConnectionPreferenceStorage {
    fun read(): String?
    fun write(encoded: String): Boolean
}

/** Application-owned admission: one original storage operation, no waiting queue.
 * Cancellation retains its exact token until the platform call actually returns.
 * Eventual release assumes that call returns; a stuck call keeps its funded slot. */
internal class ConnectionPreferenceStore(
    private val storage: ConnectionPreferenceStorage,
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
) {
    private val gate = Mutex()
    private var uncertainPublication = false

    suspend fun load(): Result<ConnectionWorkerSpec> = access {
        check(!uncertainPublication) { "USB settings save was not confirmed. Save settings again before connecting." }
        val encoded = storage.read()
        if (encoded == null) ConnectionWorkerSpec("usb") else ConnectionPreferences.decode(encoded)
    }

    suspend fun save(fields: Map<String, String>): Result<ConnectionWorkerSpec> {
        currentCoroutineContext().ensureActive()
        val selected = try { ConnectionPreferences.parse(fields) }
        catch (failure: IllegalArgumentException) { return Result.failure(failure) }
        return access {
            uncertainPublication = true
            check(storage.write(ConnectionPreferences.encode(selected))) { "USB settings could not be saved." }
            uncertainPublication = false
            selected
        }
    }

    private suspend fun <T> access(operation: () -> T): Result<T> {
        currentCoroutineContext().ensureActive()
        val original = Any()
        if (!gate.tryLock(original)) return Result.failure(IllegalStateException("USB settings storage is busy. Retry after it finishes."))
        try {
            return withContext(NonCancellable + dispatcher) {
                try { Result.success(operation()) }
                catch (cancelled: CancellationException) { throw cancelled }
                catch (failure: Exception) { Result.failure(failure) }
            }
        } finally { gate.unlock(original) }
    }
}
