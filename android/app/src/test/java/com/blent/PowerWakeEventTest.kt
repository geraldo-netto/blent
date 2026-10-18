package com.blent

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class PowerWakeEventTest {
    @Test fun t1060ReadinessValuesRoundTripWithoutCarryingAuthority() {
        for (cpu in listOf(false, true)) {
            for (wifi in listOf(false, true)) {
                val event = WorkerEvent.WakeReady(cpu, wifi)
                val encoded = WorkerEventJson.encode(event)
                assertTrue(encoded.length < 128)
                assertEquals(event, WorkerEventJson.decode(encoded))
            }
        }
    }
    @Test fun t1060ReadinessRejectsMalformedMissingAndExtraFields() {
        val original = WorkerEventJson.encode(WorkerEvent.WakeReady(true, false))
        for (field in listOf("cpu", "wifi", "type")) {
            for (invalid in listOf(JSONObject.NULL, "true", -1, 0, Long.MAX_VALUE, 1.25, JSONObject(), JSONArray())) {
                assertTrue(runCatching { WorkerEventJson.decode(JSONObject(original).put(field, invalid).toString()) }.isFailure)
            }
            assertTrue(runCatching { WorkerEventJson.decode(JSONObject(original).apply { remove(field) }.toString()) }.isFailure)
        }
        assertTrue(runCatching { WorkerEventJson.decode(JSONObject(original).put("extra", false).toString()) }.isFailure)
        assertTrue(runCatching { WakeTaskJson.decodeReady(JSONObject(original).put("type", "status")) }.isFailure)
        for (length in original.indices) assertTrue(runCatching { WorkerEventJson.decode(original.take(length)) }.isFailure)
    }
}
