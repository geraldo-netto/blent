package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class BoundedPeerCopyTest {
    private fun peer(index: Int) = ConnectionPeer.parse(index.toString(16).padStart(64, '0'))
    @Test fun t1153BoundedCopiesPreserveEveryAdmittedCountAndRejectBeforeAccess() {
        val source = (0 until 1024).map(::peer)
        for (count in 0..1024) {
            val expected = source.take(count)
            assertEquals(expected, boundedPeerCopy(expected, 1024))
        }
        for (maximum in listOf(-1, 1025, Int.MIN_VALUE, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { boundedPeerCopy(emptyList(), maximum) }
        }
        for (maximum in listOf(0, 1, 48, 1024)) {
            var reads = 0
            val excessive = object : AbstractList<ConnectionPeer>() {
                override val size: Int get() = maximum + 1
                override fun get(index: Int): ConnectionPeer { reads++; error("unadmitted read") }
            }
            assertThrows(IllegalArgumentException::class.java) { boundedPeerCopy(excessive, maximum) }
            assertEquals(0, reads)
        }
    }
    @Test fun t1153FailedAdmittedCopyCannotPublishPartialOutput() {
        var reads = 0
        val source = object : AbstractList<ConnectionPeer>() {
            override val size: Int get() = 2
            override fun get(index: Int): ConnectionPeer {
                reads++
                check(index == 0) { "injected read failure" }
                return peer(1)
            }
        }
        assertThrows(IllegalStateException::class.java) { boundedPeerCopy(source, 2) }
        assertEquals(2, reads)
    }
}
