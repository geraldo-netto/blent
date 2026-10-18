package com.blent

import org.junit.Assert.*
import org.junit.Test

class WakeLeaseTest {
    private class Handle : WakeHandle {
        var live = false
        var acquired = 0
        var retired = 0
        var acquireError: Throwable? = null
        var retireError: Throwable? = null
        var duringAcquire: () -> Unit = {}
        override fun acquire() {
            acquired++; live = true; duringAcquire()
            acquireError?.let { throw it }
        }
        override fun retire() {
            retired++
            retireError?.let { throw it }
            live = false
        }
    }
    @Test fun t1063FailedOriginalRetirementCannotReleaseOwnershipOrAllocateReplacement() {
        val handle = Handle()
        var allocated = 0
        val owner = WakeLease { allocated++; handle }
        owner.request(true)
        handle.retireError = IllegalStateException("T1063 actual retirement missing")
        assertTrue(runCatching { owner.retire() }.isFailure)
        assertFalse("T1063 release failure erased original wake ownership", owner.settled)
        assertTrue(handle.live)
        assertTrue(runCatching { owner.request(true) }.isFailure)
        assertEquals(1, allocated); assertEquals(1, handle.acquired)
        handle.retireError = null
        owner.request(false)
        assertTrue(owner.settled); assertFalse(handle.live)
    }
    @Test fun t1063AcquireFailureRetainsExactHandleUntilItsActualRetirement() {
        val handle = Handle().apply { acquireError = IllegalStateException("T1063 uncertain acquire") }
        var allocated = 0
        val owner = WakeLease { allocated++; handle }
        assertTrue(runCatching { owner.request(true) }.isFailure)
        assertFalse(owner.settled); assertTrue(handle.live)
        assertTrue(runCatching { owner.request(true) }.isFailure)
        assertEquals(1, allocated)
        owner.retire()
        assertTrue(owner.settled); assertFalse(handle.live)
    }
    @Test fun t1063FactoryFailureAdmitsNoGrantAndEachSettledSuccessorUsesFreshHandle() {
        val handles = ArrayList<Handle>()
        var reject = true
        val owner = WakeLease {
            if (reject) error("T1063 factory has admitted no native work")
            Handle().also { handles.add(it) }
        }
        assertTrue(runCatching { owner.request(true) }.isFailure); assertTrue(owner.settled)
        owner.retire(); owner.request(false)
        reject = false
        owner.request(true); owner.request(true)
        assertEquals(1, handles.size)
        owner.retire(); owner.retire()
        owner.request(true)
        assertEquals(2, handles.size)
        assertFalse(handles.first().live); assertTrue(handles.last().live)
        owner.retire()
        assertTrue(handles.none { it.live })
    }
    @Test fun t1063FatalEffectsPropagateWithoutLosingTheirOriginalCleanupSponsor() {
        val handle = Handle()
        val fatal = LinkageError("T1063 deliberate fatal effect")
        val owner = WakeLease { handle }
        handle.acquireError = fatal
        assertSame(fatal, runCatching { owner.request(true) }.exceptionOrNull())
        handle.retireError = fatal
        assertSame(fatal, runCatching { owner.retire() }.exceptionOrNull())
        assertFalse(owner.settled)
        handle.retireError = null
        owner.retire(); assertTrue(owner.settled)
    }
    @Test fun t1063ReentrantFactoryAndNativeEffectsCannotReplaceOrPrematurelyRetireOriginal() {
        lateinit var owner: WakeLease
        val handle = Handle()
        owner = WakeLease {
            assertFalse("T1063 in-flight factory falsely reported settlement", owner.settled)
            assertTrue(runCatching { owner.request(false) }.isFailure)
            handle
        }
        handle.duringAcquire = { assertFalse(owner.settled); assertTrue(runCatching { owner.retire() }.isFailure) }
        owner.request(true)
        assertTrue(handle.live); assertEquals(1, handle.acquired)
        owner.retire(); assertTrue(owner.settled)
    }
}
