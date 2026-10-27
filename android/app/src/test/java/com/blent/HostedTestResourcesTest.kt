package com.blent

import org.junit.Assert.assertTrue
import org.junit.Test

class HostedTestResourcesTest {
    @Test fun t1302WorkerReceivesItsDeclaredHeapBudget() {
        val expectedMiB = System.getProperty("blent.test.heapMiB")?.toLong() ?: 2048L
        val actual = Runtime.getRuntime().maxMemory()
        val allowance = ((expectedMiB - 8) * 1048576)..((expectedMiB + 8) * 1048576)
        assertTrue("T1302 worker max heap $actual does not match $expectedMiB MiB", actual in allowance)
    }
}
