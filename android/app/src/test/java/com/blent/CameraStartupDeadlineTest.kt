package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class CameraStartupDeadlineTest {
    @Test fun t806_missingCallbacksReportStallUntilEveryLateLeaseReturns() = runBlocking {
        val owner = CameraResources(NativeDeadline("Camera startup", 20))
        val child = owner.child()
        val first = child.startup()
        val second = child.startup()
        try {
            assertNotNull("T806 callback admission has no independent deadline", owner.startupStatus.value)
            withTimeout(2000) { while (owner.startupStatus.value?.contains("stalled") != true) delay(1) }
            first(); first()
            assertNotNull(owner.startupStatus.value)
            child.cancel()
            assertTrue(owner.startupStatus.value!!.contains("stalled"))
            second()
            assertNull(owner.startupStatus.value)
        } finally { first(); second(); child.close(); owner.close() }
    }
}
