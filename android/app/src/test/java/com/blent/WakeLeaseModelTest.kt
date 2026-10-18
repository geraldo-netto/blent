package com.blent

import org.junit.Assert.*
import org.junit.Test

class WakeLeaseModelTest {
    private enum class Step { WANT, STOP, FAIL_ACQUIRE, ALLOW_ACQUIRE, FAIL_RETIRE, ALLOW_RETIRE }
    private class Faults(var acquire: Boolean = false, var retire: Boolean = false)
    private class Handle(private val faults: Faults) : WakeHandle {
        var live = false
        override fun acquire() {
            live = true // Worst case: failure after the native grant.
            check(!faults.acquire) { "T1063 injected uncertain acquisition" }
        }
        override fun retire() {
            check(!faults.retire) { "T1063 injected unconfirmed retirement" }
            live = false
        }
    }
    private class Trace {
        private val faults = Faults()
        private val handles = ArrayList<Handle>()
        private val owner = WakeLease { Handle(faults).also { handles.add(it) } }
        private var admitted = false
        private var uncertain = false
        private var allocations = 0

        fun step(step: Step) {
            when (step) {
                Step.WANT -> acquire()
                Step.STOP -> retire()
                Step.FAIL_ACQUIRE -> faults.acquire = true
                Step.ALLOW_ACQUIRE -> faults.acquire = false
                Step.FAIL_RETIRE -> faults.retire = true
                Step.ALLOW_RETIRE -> faults.retire = false
            }
            assertEquals(!admitted, owner.settled)
            assertEquals(allocations, handles.size)
            assertEquals(if (admitted) 1 else 0, handles.count { it.live })
        }
        private fun acquire() {
            val failed = if (admitted) uncertain else faults.acquire
            val result = runCatching { owner.request(true) }
            if (!admitted) {
                allocations++; admitted = true; uncertain = faults.acquire
            }
            assertEquals(failed, result.isFailure)
        }
        private fun retire() {
            val failed = admitted && faults.retire
            val result = runCatching { owner.request(false) }
            if (failed) uncertain = true
            else { admitted = false; uncertain = false }
            assertEquals(failed, result.isFailure)
        }
        fun finish() {
            faults.retire = false
            try { owner.retire() }
            finally { handles.filter { it.live }.forEach { it.retire() } }
            assertTrue(owner.settled)
            assertTrue(handles.none { it.live })
        }
    }
    @Test fun t1063All1296FourStepHistoriesConserveOriginalHandleThroughAdmissionAndRetirementFaults() {
        repeat(1296) { encoded ->
            val trace = Trace()
            try {
                var remaining = encoded
                repeat(4) {
                    trace.step(Step.entries[remaining % 6])
                    remaining /= 6
                }
            } finally { trace.finish() }
        }
    }
}
