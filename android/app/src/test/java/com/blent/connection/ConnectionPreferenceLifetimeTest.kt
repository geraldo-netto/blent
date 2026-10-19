package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionPreferenceLifetimeTest {
    @Test fun t1099AlreadyCancelledSaveCannotPublishAValidationResult() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        var returned = false
        val caller = launch(start = CoroutineStart.UNDISPATCHED) {
            currentCoroutineContext().cancel()
            store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb")) + ("fallback_ms" to "0"))
            returned = true
        }
        caller.join()
        assertFalse("A cancelled caller received a validation result", returned)
        assertEquals(0, storage.writes)
    }

    @Test fun t1099CancellationRetainsOriginalReadAndWriteUntilActualReturn() = runBlocking {
        for (write in listOf(false, true)) PreferenceStorageRig(write).use { rig ->
            val original = rig.begin()
            rig.awaitAdmission()
            original.cancel()
            assertFalse(original.isCompleted)
            repeat(64) {
                assertTrue(rig.store.load().exceptionOrNull()?.message.orEmpty().contains("busy"))
                assertTrue(rig.store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))).isFailure)
            }
            assertEquals(if (write) 0 else 1, rig.reads.get())
            assertEquals(if (write) 1 else 0, rig.writes.get())
            rig.settle()
            original.join()
            assertTrue(original.isCancelled)
            val expected = if (write) 1000L else 100L
            assertEquals(expected, rig.store.load().getOrThrow().timerMs)
        }
    }

    @Test fun t1099CancelledBeforeAdmissionPerformsNoStorageEffect() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        val original = launch(start = CoroutineStart.LAZY) { store.load() }
        original.cancelAndJoin()
        assertEquals(0, storage.reads)
        assertEquals(0, storage.writes)
        assertTrue(store.load().isSuccess)
    }

    @Test fun t1099NativeCancellationPropagatesAndReleasesOnlySettledCall() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        storage.readFailure = CancellationException("native read cancellation")
        val read = async { store.load() }
        read.join()
        assertTrue(read.isCancelled)
        storage.readFailure = null
        storage.writeFailure = CancellationException("native write cancellation")
        val write = async { store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))) }
        write.join()
        assertTrue(write.isCancelled)
        assertTrue(store.load().isFailure)
        storage.writeFailure = null
        assertTrue(store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))).isSuccess)
        assertTrue(store.load().isSuccess)
    }
}
