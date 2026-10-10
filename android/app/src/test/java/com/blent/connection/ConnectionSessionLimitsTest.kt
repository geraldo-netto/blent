package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionSessionLimitsTest {
    @Test fun t882CapacityOptionsBoundEveryIndependentResource() {
        val defaults = ConnectionSessionLimits()
        assertEquals(262144, defaults.capacity); assertEquals(64, defaults.messages)
        assertEquals(65536, defaults.quantum); assertEquals(120000L, defaults.resourceTimeoutMs)
        for (capacity in listOf(4096, ConnectionWire.MAX_CREDIT)) {
            for (messages in listOf(1, 1024)) {
                for (quantum in listOf(1, ConnectionWire.MAX_PAYLOAD)) {
                    val limits = ConnectionSessionLimits(capacity, messages, quantum, 600000)
                    assertEquals(capacity, limits.capacity); assertEquals(messages, limits.messages)
                    assertEquals(quantum, limits.quantum)
                }
            }
        }
        assertEquals(1L, ConnectionSessionLimits(resourceTimeoutMs = 1).resourceTimeoutMs)
    }
    private fun reject(make: (Int) -> Unit, values: List<Int>) {
        for (value in values) assertThrows(IllegalArgumentException::class.java) { make(value) }
    }
    @Test fun t882RejectsExtremeCapacityAndDeadlineValues() {
        reject({ ConnectionSessionLimits(capacity = it) }, listOf(Int.MIN_VALUE, -1, 0, 4095, 1048577, Int.MAX_VALUE))
        reject({ ConnectionSessionLimits(messages = it) }, listOf(Int.MIN_VALUE, -1, 0, 1025, Int.MAX_VALUE))
        reject({ ConnectionSessionLimits(quantum = it) }, listOf(Int.MIN_VALUE, -1, 0, 65537, Int.MAX_VALUE))
        for (timeout in listOf(Long.MIN_VALUE, -1, 0, 600001, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionSessionLimits(resourceTimeoutMs = timeout) }
        }
    }
}
