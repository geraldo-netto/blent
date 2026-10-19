package com.blent.connection

import android.content.Context
import android.content.ContextWrapper
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AndroidConnectionPreferencesTest {
    @Test fun t1099PrivateAdapterPersistsAcrossOwnersAndPreservesCorruptionEvidence() = runBlocking {
        val context = RuntimeEnvironment.getApplication()
        val preferences = context.getSharedPreferences("usb_connection", Context.MODE_PRIVATE)
        assertTrue(preferences.edit().clear().commit())
        val store = ConnectionPreferenceStore(AndroidConnectionPreferenceStorage(ContextWrapper(context)))
        assertEquals(ConnectionWorkerSpec("usb"), store.load().getOrThrow())
        val expected = ConnectionWorkerSpec("usb", timerMs = 333, session = ConnectionSessionLimits(capacity = 4096))
        assertEquals(expected, store.save(ConnectionPreferences.fields(expected)).getOrThrow())
        assertEquals(ConnectionPreferences.encode(expected), preferences.getString("settings", null))
        val reopened = ConnectionPreferenceStore(AndroidConnectionPreferenceStorage(context))
        assertEquals(expected, reopened.load().getOrThrow())
        assertTrue(preferences.edit().putString("settings", "broken").commit())
        assertTrue(reopened.load().isFailure)
        assertEquals("broken", preferences.getString("settings", null))
        assertTrue(preferences.edit().putInt("settings", 1).commit())
        assertTrue(reopened.load().isFailure)
    }

    @Test fun t1099AdapterRejectsOversizedWritesBeforeChangingPrivateRecord() = runBlocking {
        val context = RuntimeEnvironment.getApplication()
        val preferences = context.getSharedPreferences("usb_connection", Context.MODE_PRIVATE)
        assertTrue(preferences.edit().clear().commit())
        val storage = AndroidConnectionPreferenceStorage(context)
        withContext(Dispatchers.IO) {
            assertThrows(IllegalArgumentException::class.java) { storage.write("") }
            val random = java.util.Random(1099)
            repeat(64) {
                val text = "1".repeat(513 + random.nextInt(512))
                assertThrows(IllegalArgumentException::class.java) { storage.write(text) }
            }
            assertNull(storage.read())
        }
    }

    @Test fun t1099ApplicationOwnerIsSingleAndBorrowsOnlyApplicationContext() = runBlocking {
        val context = RuntimeEnvironment.getApplication()
        val previous = ReflectionHelpers.getStaticField<ConnectionPreferenceStore?>(AndroidConnectionPreferences::class.java, "store")
        try {
            ReflectionHelpers.setStaticField(AndroidConnectionPreferences::class.java, "store", null)
            val first = AndroidConnectionPreferences.get(ContextWrapper(context))
            assertSame(first, AndroidConnectionPreferences.get(context))
            val storage = ReflectionHelpers.getField<AndroidConnectionPreferenceStorage>(first, "storage")
            assertSame(context, ReflectionHelpers.getField<Context>(storage, "context"))
            context.getSharedPreferences("usb_connection", Context.MODE_PRIVATE).edit().clear().commit()
            assertTrue(first.load().isSuccess)
        } finally { ReflectionHelpers.setStaticField(AndroidConnectionPreferences::class.java, "store", previous) }
    }
    @Test fun t898SharedBoundarySnapshotsPersistAcrossPrivateStoreOwners() = runBlocking {
        val context = RuntimeEnvironment.getApplication()
        val preferences = context.getSharedPreferences("usb_connection", Context.MODE_PRIVATE)
        assertTrue(preferences.edit().clear().commit())
        val store = ConnectionPreferenceStore(AndroidConnectionPreferenceStorage(context))
        val current = ConnectionWorkerSpec("usb")
        val fields = ConnectionPreferences.fields(current)
        for (range in usbSettingsContract()) for (value in listOf(range.minimum, range.maximum)) {
            val selected = store.save(fields + (range.field to value.toString())).getOrThrow()
            val reopened = ConnectionPreferenceStore(AndroidConnectionPreferenceStorage(context))
            assertEquals(selected, reopened.load().getOrThrow())
            assertEquals(ConnectionPreferences.encode(selected), preferences.getString("settings", null))
            assertEquals(fields, ConnectionPreferences.fields(current))
        }
    }
}
