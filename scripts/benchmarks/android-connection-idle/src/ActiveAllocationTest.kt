package com.blent.connection

import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class ActiveAllocationTest {
    @Test fun t1768CurrentActorAcknowledgesEveryCueAndRetiresItsOriginalThread() {
        for (fps in listOf(0, 60)) {
            val result = activeActorSample(fps, 2, 4)
            assertEquals(4, result.getValue("events").jsonPrimitive.int)
            assertEquals(4, result.getValue("latency_ns").jsonArray.size)
            assertTrue(result.getValue("advances").jsonPrimitive.long >= 4)
            assertTrue(result.getValue("actor_cpu_ns").jsonPrimitive.long > 0)
            assertTrue(result.getValue("actor_allocated_bytes").jsonPrimitive.long > 0)
        }
    }
    @Test fun t1768ActiveProbeRejectsInvalidFiniteDomainsBeforeActorAdmission() {
        for (fps in listOf(Int.MIN_VALUE, -1, 1, 61, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { activeActorSample(fps, 0, 1) }
        }
        for (warmup in listOf(Int.MIN_VALUE, -1, 2001, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { activeActorSample(0, warmup, 1) }
        }
        for (events in listOf(Int.MIN_VALUE, -1, 0, 10001, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { activeActorSample(0, 0, events) }
        }
    }
}
