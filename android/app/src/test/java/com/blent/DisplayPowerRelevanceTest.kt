package com.blent

import org.junit.Assert.*
import org.junit.Test

class DisplayPowerRelevanceTest {
    @Test fun t1069RelevanceCannotReplaceCurrentOwnerAndWithdrawalNeverReadsLostSource() {
        val owner = DisplayPowerOwner()
        var reads = 0
        val unavailable = DisplayPowerPermit { reads++; false }
        assertFalse(DisplayPowerRequest(owner, StreamingPower(), unavailable).permitsPower())
        assertEquals(1, reads)
        assertTrue(DisplayPowerRequest(owner, null, unavailable).permitsPower())
        assertEquals("T1069 withdrawal borrowed a lost source", 1, reads)
        assertTrue(DisplayPowerRequest(owner, StreamingPower()).permitsPower())
        assertTrue(DisplayPowerDemand(StreamingPower()).permit.current())
    }
    @Test fun t1069InvalidReaderFailsClosedAndFatalFailureRetainsItsIdentity() {
        val owner = DisplayPowerOwner()
        val failed = DisplayPowerPermit { throw IllegalStateException("T1069 original reader failed") }
        assertFalse(DisplayPowerRequest(owner, StreamingPower(), failed).permitsPower())
        val fatal = AssertionError("T1069 fatal source read")
        val demand = DisplayPowerRequest(owner, StreamingPower(), DisplayPowerPermit { throw fatal })
        assertSame(fatal, runCatching { demand.permitsPower() }.exceptionOrNull())
    }
}
