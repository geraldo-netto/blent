package com.blent

import android.content.Context
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class StreamOverridesTest {
    @Test fun t1694SparseValuesRoundTripAndMergeWithoutAddingAbsentFields() {
        val app = RuntimeEnvironment.getApplication()
        val prefs = Prefs(app)
        val choices = listOf(StreamOverrides(), StreamOverrides(1000), StreamOverrides(fps = 10), StreamOverrides(60000, 90))
        for (original in choices) {
            prefs.streamOverrides = original
            assertEquals(original, Prefs(app).streamOverrides)
            assertEquals(original.empty, original.bitrateKbps == null && original.fps == null)
            for (change in choices) assertMerged(original, change)
            assertEquals(StreamOverrides(original.bitrateKbps?.let { 25000 }, original.fps?.let { 30 }),
                original.confirmed(HostStreamSettings(25000, 30)))
        }
        prefs.streamOverrides = StreamOverrides()
        assertTrue(Prefs(app).streamOverrides.empty)
    }

    private fun assertMerged(original: StreamOverrides, change: StreamOverrides) {
        val merged = original.merge(change)
        assertEquals(change.bitrateKbps ?: original.bitrateKbps, merged.bitrateKbps)
        assertEquals(change.fps ?: original.fps, merged.fps)
        if (merged.empty) return
        val wire = StreamOverridesJson.config(merged)
        assertEquals(merged, StreamOverridesJson.read(wire))
        assertEquals(merged.bitrateKbps != null, wire.has("bitrate"))
        assertEquals(merged.fps != null, wire.has("fps"))
    }

    @Test fun t1694DraftDropsOnlyFieldsEqualToCurrentHostValues() {
        assertTrue(StreamOverrides(20000, 60).differentFrom(20000, 60).empty)
        assertEquals(StreamOverrides(fps = 30), StreamOverrides(20000, 30).differentFrom(20000, 60))
        assertEquals(StreamOverrides(25000), StreamOverrides(25000, 60).differentFrom(20000, 60))
        assertTrue(StreamOverrides().differentFrom(20000, 60).empty)
        assertThrows(IllegalArgumentException::class.java) { StreamOverridesJson.config(StreamOverrides()) }
    }

    @Test fun t1694InvalidSparseWireFieldsNeverBecomeDefaults() {
        for (field in listOf("bitrate", "fps")) {
            for (value in listOf(-1, 0, 1.5, "20", true, 2147483648L)) {
                assertNull("$field=$value", StreamOverridesJson.read(JSONObject().put(field, value)))
            }
        }
        for ((bitrate, fps) in listOf(999 to 60, 60001 to 60, 20000 to 9, 20000 to 91)) {
            assertThrows(IllegalArgumentException::class.java) { StreamOverrides(bitrate, fps) }
            assertNull(StreamOverridesJson.read(JSONObject().put("bitrate", bitrate).put("fps", fps)))
        }
        assertEquals(StreamOverrides(), StreamOverridesJson.read(JSONObject()))
        assertEquals(StreamOverrides(fps = 30), StreamOverridesJson.read(JSONObject().put("bitrate", JSONObject.NULL).put("fps", 30)))
    }

    @Test fun t1694InvalidStoredFieldsAreAbsentAndOldLayoutIsNotMigrated() {
        val app = RuntimeEnvironment.getApplication()
        val store = app.getSharedPreferences("blent", Context.MODE_PRIVATE)
        store.edit().clear().putInt("bitrate_kbps", 60000).putInt("fps", 90).putBoolean("has_user_settings", true).commit()
        assertTrue(Prefs(app).streamOverrides.empty)
        store.edit().putString("stream_override_bitrate", "20000").putInt("stream_override_fps", 100).commit()
        assertTrue(Prefs(app).streamOverrides.empty)
        Prefs(app).streamOverrides = StreamOverrides(25005, 60)
        assertEquals(StreamOverrides(25005, 60), Prefs(app).streamOverrides)
    }
}
