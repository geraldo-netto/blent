package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class TrustSnapshotAdmissionTest {
    @Test fun t1154RejectedPersistedListIsNeverCopiedBeforeAdmission() {
        var reads = 0
        val oversized = object : AbstractList<ConnectionPeer>() {
            override val size: Int get() = 1025
            override fun get(index: Int): ConnectionPeer { reads++; error("unadmitted persisted peer read") }
        }
        val local = ConnectionPeer.parse("a".repeat(64))
        assertThrows(IllegalArgumentException::class.java) { TrustSnapshot(local, 1, oversized) }
        assertEquals("T1154 rejected list was accessed", 0, reads)
    }
}
