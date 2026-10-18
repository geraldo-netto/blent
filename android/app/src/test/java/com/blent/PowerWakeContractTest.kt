package com.blent

import kotlinx.coroutines.runBlocking
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class PowerWakeContractTest {
    @Test fun t1060EveryWakeTaskRoundTripsAndUsesOnlyItsPrivateWorker() = runBlocking {
        for (cpu in listOf(false, true)) {
            for (wifi in listOf(false, true)) {
                val task = MediaTask.Wake(cpu, wifi)
                val encoded = MediaTaskJson.encode(task)
                assertTrue(encoded.length < 128)
                assertEquals(task, MediaTaskJson.decode(encoded))
                assertTrue(PowerWorkerService().accepts(task))
                assertFalse(MetadataWorkerService().accepts(task))
                val run = AndroidWorkerRun(RuntimeEnvironment.getApplication(), task)
                try { assertEquals(PowerWorkerService::class.java, run.serviceClass(task)) }
                finally { run.retire() }
            }
        }
        assertFalse(PowerWorkerService().accepts(MediaTask.Metadata(1, 1, 1)))
    }
    @Test fun t1060MalformedWakeTaskNeverCoercesTypesOrAdmitsExtraFields() {
        val original = MediaTaskJson.encode(MediaTask.Wake(true, false))
        for (field in listOf("cpu", "wifi", "version", "kind")) {
            for (invalid in listOf(JSONObject.NULL, "true", "false", -1, 0, Long.MAX_VALUE, 1.25, JSONObject(), org.json.JSONArray())) {
                assertTrue("T1060 $field=$invalid", runCatching { MediaTaskJson.decode(JSONObject(original).put(field, invalid).toString()) }.isFailure)
            }
            assertTrue(runCatching { MediaTaskJson.decode(JSONObject(original).apply { remove(field) }.toString()) }.isFailure)
        }
        assertTrue(runCatching { MediaTaskJson.decode(JSONObject(original).put("extra", true).toString()) }.isFailure)
        assertTrue(runCatching { WakeTaskJson.decode(JSONObject(original).put("kind", "metadata")) }.isFailure)
        for (length in 0 until original.length) assertTrue(runCatching { MediaTaskJson.decode(original.take(length)) }.isFailure)
    }
}
