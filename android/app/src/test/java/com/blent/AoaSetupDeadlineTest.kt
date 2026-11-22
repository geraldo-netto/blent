package com.blent

import org.junit.Assert.*
import org.junit.Test

class AoaSetupDeadlineTest {
    @Test fun t1726EveryReadUsesRemainingOriginalBudget() {
        for (elapsed in 0L until 120000L) {
            assertEquals((120000L - elapsed).toInt(),
                aoaSetupRemaining(121000L, 1000L + elapsed, false))
        }
    }

    @Test fun t1726CancelledOrExpiredSetupCannotBecomeInfiniteRead() {
        for (now in listOf(121000L, 121001L, Long.MAX_VALUE)) {
            assertThrows(IllegalStateException::class.java) {
                aoaSetupRemaining(121000L, now, false)
            }
        }
        assertThrows(IllegalStateException::class.java) {
            aoaSetupRemaining(121000L, 1000L, true)
        }
    }

    @Test fun t1726InvalidClockInputsAndOversizedBudgetsAreRejected() {
        for ((deadline, now) in listOf(-1L to 0L, 1L to -1L, Long.MIN_VALUE to Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) {
                aoaSetupRemaining(deadline, now, false)
            }
        }
        assertThrows(IllegalStateException::class.java) {
            aoaSetupRemaining(Long.MAX_VALUE, 0L, false)
        }
        assertEquals(1, aoaSetupRemaining(Long.MAX_VALUE, Long.MAX_VALUE - 1, false))
    }
}
