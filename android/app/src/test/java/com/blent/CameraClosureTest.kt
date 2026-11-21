package com.blent

import java.util.concurrent.*
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Test

class CameraClosureTest {
    @Test fun t1462_onlyOriginalIdentityCanRetireAllWaitingClosers() {
        val entered = CountDownLatch(1)
        val calls = AtomicInteger()
        val closure = CameraClosure<Any> { calls.incrementAndGet(); entered.countDown() }
        val original = Any()
        val foreign = Any()
        closure.bind(original)
        closure.bind(original)
        assertThrows(IllegalStateException::class.java) { closure.bind(foreign) }
        assertThrows(IllegalStateException::class.java) { closure.close(foreign) }
        closure.completed(foreign)
        val executor = Executors.newFixedThreadPool(2)
        val first = executor.submit { closure.close(original) }
        val second = executor.submit { closure.close(original) }
        try {
            assertTrue(entered.await(5, TimeUnit.SECONDS))
            assertThrows(TimeoutException::class.java) { first.get(100, TimeUnit.MILLISECONDS) }
            assertThrows(TimeoutException::class.java) { second.get(100, TimeUnit.MILLISECONDS) }
            closure.completed(original)
            first.get(5, TimeUnit.SECONDS); second.get(5, TimeUnit.SECONDS)
            closure.completed(original)
            closure.close(original)
            assertEquals(1, calls.get())
        } finally { closure.completed(original); executor.shutdown(); executor.awaitTermination(5, TimeUnit.SECONDS) }
    }

    @Test fun t1462_closeFailureDoesNotFabricateRetirementOrRepeatNativeRequest() {
        val original = Any()
        var calls = 0
        val closure = CameraClosure<Any> { calls++; error("native close failed") }
        closure.bind(original)
        assertThrows(IllegalStateException::class.java) { closure.close(original) }
        closure.completed(original)
        closure.close(original)
        assertEquals(1, calls)
    }
}
