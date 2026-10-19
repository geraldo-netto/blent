package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionPreferenceFormTest {
    @Test fun t1099DraftValidationSaveReloadAndDefaultsPreservePublicationBoundaries() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        val form = ConnectionPreferenceForm()
        form.load(store)
        form.edit(ConnectionPreferenceField.FALLBACK, "0")
        assertTrue(form.invalid(ConnectionPreferenceField.FALLBACK))
        form.save(store)
        assertNotNull(form.error)
        assertEquals(0, storage.writes)
        form.edit(ConnectionPreferenceField.FALLBACK, "250")
        assertFalse(form.invalid(ConnectionPreferenceField.FALLBACK))
        form.save(store)
        assertEquals("Saved for the next USB connection.", form.notice)
        assertEquals(250, store.load().getOrThrow().timerMs)
        form.defaults()
        assertEquals("100", form.fields["fallback_ms"])
        assertEquals(250, store.load().getOrThrow().timerMs)
        form.load(store)
        assertEquals("250", form.fields["fallback_ms"])
        assertNull(form.notice)
    }

    @Test fun t1099FailedAndUnconfirmedWritesRequireExplicitSuccessfulRetry() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        val form = ConnectionPreferenceForm()
        form.edit(ConnectionPreferenceField.CHANNEL_BYTES, "4096")
        storage.accepted = false
        form.save(store)
        assertNotNull(form.error)
        assertNull(form.notice)
        form.load(store)
        assertTrue(form.error.orEmpty().contains("not confirmed"))
        assertEquals("4096", form.fields["channel_bytes"])
        storage.accepted = true
        form.save(store)
        assertNull(form.error)
        assertEquals(4096, store.load().getOrThrow().session.capacity)
    }

    @Test fun t1099BusyAndCancelledOriginalRequestCannotEditOrPublishLateDraft() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val dispatcher = ConnectionActorScheduler()
        val store = ConnectionPreferenceStore(storage, dispatcher)
        val form = ConnectionPreferenceForm()
        val original = async(start = CoroutineStart.UNDISPATCHED) { form.load(store) }
        try {
            assertTrue(form.busy)
            form.edit(ConnectionPreferenceField.FALLBACK, "1"); form.defaults()
            form.load(store); form.save(store)
            assertEquals("100", form.fields["fallback_ms"])
            assertEquals(0, storage.reads)
            original.cancel()
            assertFalse(original.isCompleted)
            dispatcher.runCurrent(); original.join()
            assertFalse(form.busy)
            assertEquals(1, storage.reads)
            assertEquals(0, storage.writes)
            assertEquals("100", form.fields["fallback_ms"])
        } finally { original.cancel(); dispatcher.runCurrent(); original.join() }
        assertTrue(dispatcher.empty())
    }

    @Test fun t1099OversizedEditsAndBackendErrorMessagesRemainBoundedAndVisible() = runBlocking {
        val storage = PreferenceMemoryStorage()
        val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
        val form = ConnectionPreferenceForm()
        form.edit(ConnectionPreferenceField.FALLBACK, "1".repeat(21))
        assertEquals("100", form.fields["fallback_ms"])
        assertNotNull(form.error)
        storage.readFailure = Exception()
        form.load(store)
        assertEquals("USB settings request failed.", form.error)
        storage.readFailure = Exception("x".repeat(1000))
        form.load(store)
        assertEquals(256, form.error?.length)
    }
}
