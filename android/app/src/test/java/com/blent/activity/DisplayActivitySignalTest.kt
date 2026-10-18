package com.blent.activity

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayActivitySignalTest {
    @Test fun t1055SharedWireVectorsRequireExactTypesAndRanges() {
        val input = checkNotNull(javaClass.getResourceAsStream("/display-activity-v1.json"))
        val vectors = input.bufferedReader().use { JSONObject(it.readText()) }
        val valid = vectors.getJSONArray("valid")
        for (index in 0 until valid.length()) {
            val signal = DisplayActivitySignal.read(JSONObject(valid.getJSONObject(index).getString("json")))
            assertTrue(signal.revision > 0)
            assertTrue(signal.stopped <= signal.revision)
        }
        val invalid = vectors.getJSONArray("invalid")
        for (index in 0 until invalid.length()) {
            val case = invalid.getJSONObject(index)
            assertTrue(case.getString("name"), runCatching { DisplayActivitySignal.read(JSONObject(case.getString("json"))) }.isFailure)
        }
    }
    @Test fun t1055BoundedCounterBoundariesCannotConstructInvalidSignal() {
        for (revision in listOf(Long.MIN_VALUE, -1, 0, 1, 2, Long.MAX_VALUE - 1, Long.MAX_VALUE)) {
            for (stopped in listOf(Long.MIN_VALUE, -1, 0, 1, 2, Long.MAX_VALUE)) {
                for (active in listOf(false, true)) {
                    val valid = revision > 0 && stopped >= 0 && stopped <= revision
                    assertEquals(valid, runCatching { DisplayActivitySignal(revision, stopped, active) }.isSuccess)
                }
            }
        }
    }
}
