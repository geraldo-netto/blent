package com.blent.connection

import java.nio.channels.OverlappingFileLockException
import java.util.concurrent.Callable
import java.util.concurrent.Executors
import org.junit.Assert.*
import org.junit.Test

class TrustDirectoryOwnershipTest {
    @Test fun t1156BoundedAdmissionAndOriginalIdentity() {
        for (capacity in listOf(Int.MIN_VALUE, -1, 0, 1025, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { TrustDirectoryOwnership(capacity) }
        }
        for (capacity in listOf(1, 2, 64, 1024)) checkCapacity(capacity)
    }

    private fun checkCapacity(capacity: Int) {
        val owner = TrustDirectoryOwnership(capacity)
        val leases = List(capacity) { owner.reserve(it) }
        try {
            assertThrows(IllegalStateException::class.java) { owner.reserve(capacity) }
            for (index in leases.indices) {
                assertThrows(OverlappingFileLockException::class.java) { owner.reserve(index) }
            }
        } finally { leases.forEach { it.close() } }
        owner.reserve(capacity).close()
    }

    @Test fun t1156StaleAndForgedLeaseCannotReleaseReplacement() {
        val owner = TrustDirectoryOwnership(1)
        val old = owner.reserve("original")
        val forged = TrustDirectoryOwnership.Lease(owner, "original", Any())
        assertThrows(IllegalStateException::class.java) { forged.close() }
        assertThrows(OverlappingFileLockException::class.java) { owner.reserve("original") }
        old.close()
        owner.reserve("original").use {
            old.close()
            assertThrows(OverlappingFileLockException::class.java) { owner.reserve("original") }
            assertThrows(IllegalStateException::class.java) { owner.reserve("other") }
        }
        owner.reserve("other").close()
    }

    @Test fun t1156ConcurrentReservationHasOneOwnerUntilOriginalClose() {
        val owner = TrustDirectoryOwnership(1)
        val pool = Executors.newFixedThreadPool(8)
        val leases = try {
            pool.invokeAll(List(32) { Callable { runCatching { owner.reserve("original") }.getOrNull() } })
                .mapNotNull { it.get() }
        } finally { pool.shutdown() }
        try { assertEquals(1, leases.size) } finally { leases.forEach { it.close() } }
        // invokeAll/get joined every original operation before the next admission.
        owner.reserve("original").close()
    }
}
