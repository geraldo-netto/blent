package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

internal class PreferenceMemoryStorage : ConnectionPreferenceStorage {
    var encoded: String? = null
    var accepted = true
    var readFailure: Exception? = null
    var writeFailure: Exception? = null
    var reads = 0
    var writes = 0
    override fun read(): String? {
        reads++
        readFailure?.let { throw it }
        return encoded
    }
    override fun write(encoded: String): Boolean {
        writes++
        this.encoded = encoded // Platform caches can change before disk publication fails.
        writeFailure?.let { throw it }
        return accepted
    }
}

class ConnectionPreferenceStoreTest {
    @Test fun t1099UnconfirmedPublicationCannotBecomeNextConnectionSettings() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        val next = ConnectionWorkerSpec("usb", timerMs = 1000)
        storage.accepted = false
        assertTrue(store.save(ConnectionPreferences.fields(next)).isFailure)
        assertEquals(ConnectionPreferences.encode(next), storage.encoded)
        assertTrue("Unconfirmed cached publication must not configure a connection", store.load().isFailure)
        storage.accepted = true
        assertEquals(next, store.save(ConnectionPreferences.fields(next)).getOrThrow())
        assertEquals(next, store.load().getOrThrow())
    }

    @Test fun t1099AbsentValidCorruptAndFailedReadsRemainDistinct() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        assertEquals(ConnectionWorkerSpec("usb"), store.load().getOrThrow())
        val next = ConnectionWorkerSpec("usb", session = ConnectionSessionLimits(capacity = 4096))
        assertEquals(next, store.save(ConnectionPreferences.fields(next)).getOrThrow())
        assertEquals(next, store.load().getOrThrow())
        storage.encoded = "broken"
        assertTrue(store.load().isFailure)
        storage.readFailure = java.io.IOException("read failed")
        assertEquals("read failed", store.load().exceptionOrNull()?.message)
    }

    @Test fun t1099ValidationAndWriteFailureNeverClaimSuccessfulSave() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        val fields = ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))
        assertTrue(store.save(fields + ("fallback_ms" to "0")).isFailure)
        assertEquals(0, storage.writes)
        storage.writeFailure = java.io.IOException("write failed")
        assertEquals("write failed", store.save(fields).exceptionOrNull()?.message)
        assertTrue(store.load().isFailure)
        storage.writeFailure = null
        assertTrue(store.save(fields).isSuccess)
        assertTrue(store.load().isSuccess)
    }
}
