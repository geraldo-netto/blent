package com.blent.connection

import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class ProducerAllocationTest {
    @Test fun t1766CurrentProducerKeepsTransferredArraysOutsideNewNativeReads() {
        for (size in listOf(1, 64, 4096, 16384, 65535, 65536)) {
            val result = producerAllocationSample(size, 4)
            assertEquals(4, result.getValue("iterations").jsonPrimitive.int)
            assertEquals(size, result.getValue("payload_bytes").jsonPrimitive.int)
            assertTrue(result.getValue("driver_allocated_bytes").jsonPrimitive.long > 0)
            assertTrue(result.getValue("driver_cpu_ns").jsonPrimitive.long > 0)
        }
    }
    @Test fun t1766ProducerFixtureRejectsInvalidFiniteDomains() {
        for (size in listOf(Int.MIN_VALUE, -1, 0, 65537, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { producerAllocationSample(size, 1) }
        }
        for (iterations in listOf(Int.MIN_VALUE, -1, 0, 1025, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { producerAllocationSample(1, iterations) }
        }
    }
}
