package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class PowerWakeRunTest {
    private class Handle : WakeHandle {
        var live = false
        var acquires = 0
        var retires = 0
        var acquireFailure: Exception? = null
        var retireFailure: Exception? = null
        var afterAcquire: () -> Unit = {}
        override fun acquire() { acquires++; live = true; afterAcquire(); acquireFailure?.let { throw it } }
        override fun retire() { retires++; retireFailure?.let { throw it }; live = false }
    }
    @Test fun t1060NativeTaskAllocatesOnlyRequestedHandlesAndSettlesBoth() = runBlocking {
        for (cpuWanted in listOf(false, true)) {
            for (wifiWanted in listOf(false, true)) {
                val cpu = Handle(); val wifi = Handle()
                val run = PowerWakeRun(MediaTask.Wake(cpuWanted, wifiWanted), { cpu }, { wifi })
                val job = launch(start = CoroutineStart.UNDISPATCHED) { run.execute() }
                try {
                    assertEquals(cpuWanted, cpu.live); assertEquals(wifiWanted, wifi.live)
                    assertTrue(runCatching { run.execute() }.isFailure)
                } finally { run.cancel(); job.join(); run.retire() }
                assertFalse(cpu.live); assertFalse(wifi.live)
                assertEquals(if (cpuWanted) 1 else 0, cpu.retires)
                assertEquals(if (wifiWanted) 1 else 0, wifi.retires)
            }
        }
    }
    @Test fun t1060RetirementBeforeExecutionAndCancellationAfterLateCpuCannotAdmitWifi() = runBlocking {
        val unopened = PowerWakeRun(MediaTask.Wake(true, true), { error("T1060 retired CPU factory") }, { error("T1060 retired Wi-Fi factory") })
        unopened.retire(); unopened.execute()
        val cpu = Handle()
        lateinit var run: PowerWakeRun
        run = PowerWakeRun(MediaTask.Wake(true, true), { cpu }, { error("T1060 Wi-Fi after cutoff") })
        cpu.afterAcquire = run::cancel
        try { run.execute(); assertTrue(cpu.live) }
        finally { run.retire() }
        assertFalse(cpu.live)
    }
    @Test fun t1060PartialAdmissionAndIndependentReleaseFaultsKeepOriginalHandles() = runBlocking {
        val cpu = Handle(); val wifi = Handle()
        val run = PowerWakeRun(MediaTask.Wake(true, true), { cpu }, { wifi })
        val partial = IllegalStateException("T1060 Wi-Fi granted then failed")
        wifi.acquireFailure = partial
        assertSame(partial, runCatching { run.execute() }.exceptionOrNull())
        val cpuFailure = IllegalStateException("T1060 CPU release missing")
        val wifiFailure = IllegalStateException("T1060 Wi-Fi release missing")
        cpu.retireFailure = cpuFailure; wifi.retireFailure = wifiFailure
        assertSame(cpuFailure, runCatching { run.retire() }.exceptionOrNull())
        assertTrue(cpuFailure.suppressed.contains(wifiFailure))
        assertEquals(1, cpu.retires); assertEquals(1, wifi.retires)
        assertTrue(cpu.live); assertTrue(wifi.live)
        cpu.retireFailure = null; wifi.retireFailure = null
        run.retire()
        assertFalse(cpu.live); assertFalse(wifi.live)
    }
    @Test fun t1060RepeatedSameReleaseFailureCannotSuppressItself() = runBlocking {
        val failure = IllegalStateException("T1060 same original release failure")
        val cpu = Handle(); val wifi = Handle()
        val run = PowerWakeRun(MediaTask.Wake(true, true), { cpu }, { wifi })
        val job = launch(start = CoroutineStart.UNDISPATCHED) { run.execute() }
        run.cancel(); job.join()
        cpu.retireFailure = failure; wifi.retireFailure = failure
        assertSame(failure, runCatching { run.retire() }.exceptionOrNull())
        assertTrue(failure.suppressed.isEmpty())
        cpu.retireFailure = null; wifi.retireFailure = null; run.retire()
    }
}
