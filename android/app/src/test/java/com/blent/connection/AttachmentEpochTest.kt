package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class AttachmentEpochTest {
    @Test fun t1405ClaimsAreSingleUseAndOldCallbacksCannotCrossCutoffs() {
        val epoch = AttachmentEpoch()
        repeat(32) {
            val old = checkNotNull(epoch.candidate())
            epoch.consume()
            assertNull(epoch.candidate())
            assertFalse(epoch.claim(old))
            epoch.detached()
            val fresh = checkNotNull(epoch.candidate())
            assertFalse(epoch.claim(old))
            assertTrue(epoch.claim(fresh))
            assertFalse(epoch.claim(fresh))
            assertNull(epoch.candidate())
            epoch.detached()
        }
    }
}
