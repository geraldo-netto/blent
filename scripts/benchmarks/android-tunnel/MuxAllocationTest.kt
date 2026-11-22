package com.blent.connection

import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class MuxAllocationTest {
    @Test fun t1765ProductionMuxHasExactFiniteDataAndIdleAccounting() {
        for (data in listOf(false, true)) {
            val result = muxAllocationSample(data, 128)
            assertEquals(128, result.getValue("iterations").jsonPrimitive.int)
            assertEquals(if (data) 8192L else 0L, result.getValue("payload_bytes_sent").jsonPrimitive.long)
            assertTrue(result.getValue("driver_allocated_bytes").jsonPrimitive.long >= 0)
            assertTrue(result.getValue("driver_cpu_ns").jsonPrimitive.long > 0)
        }
    }
    @Test fun t1765AllocationDomainRejectsInvalidIterationCounts() {
        for (count in listOf(Int.MIN_VALUE, -1, 0, MUX_ALLOCATION_ITERATIONS + 1, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { muxAllocationSample(false, count) }
        }
    }
}
