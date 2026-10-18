package com.blent

import com.blent.activity.DisplayActivitySource
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class ScopedPowerPermitTest {
    private class Native : LifecycleRun {
        @Volatile var executes = 0
        val entered = CompletableDeferred<Unit>()
        var retires = 0
        override suspend fun execute() { executes++; entered.complete(Unit); awaitCancellation() }
        override fun cancel() {}
        override suspend fun retire() { retires++ }
    }
    @Test fun t1069LateCreatorCannotExecuteUntilAReplacementViewIsActuallyCurrent() = runBlocking {
        val source = DisplayActivitySource()
        val snapshot = source.update
        val invalidRead = CompletableDeferred<Unit>()
        val permit = DisplayPowerPermit { source.current(snapshot).also { if (!it) invalidRead.complete(Unit) } }
        val channel = DisplayPowerRequests()
        val admission = channel.open(StreamingPower(), permit)
        val request = checkNotNull(channel.receive(checkNotNull(admission.notice)).request)
        val stamp = Any()
        val views = MutableStateFlow<PowerWorkerView?>(PowerWorkerView(request, MediaTask.Wake(true, true), stamp))
        val creating = CompletableDeferred<Unit>()
        val release = java.util.concurrent.CountDownLatch(1)
        val native = Native()
        val run = ScopedPowerRun(views, stamp, AtomicBoolean(), channel) { creating.complete(Unit); release.await(); native }
        val job = launch(Dispatchers.Default) { run.execute() }
        try {
            withTimeout(2000) { creating.await() }
            source.replace(); release.countDown()
            val checked = withTimeoutOrNull(2000) { invalidRead.await(); true } == true
            assertTrue("T1069 late creator skipped source revalidation; native executions=${native.executes}", checked)
            assertEquals(0, native.executes)
            val fresh = source.update
            val notice = checkNotNull(channel.publish(admission.owner, StreamingPower(), DisplayPowerPermit { source.current(fresh) }))
            views.value = PowerWorkerView(checkNotNull(channel.receive(notice).request), MediaTask.Wake(true, true), stamp)
            withTimeout(2000) { native.entered.await() }
            assertEquals(1, native.executes)
        } finally { release.countDown(); run.cancel(); job.cancelAndJoin(); run.retire() }
        assertEquals(1, native.retires)
    }
    @Test fun t1069InvalidOriginalViewWaitsForFreshCurrentDemandBeforeLazyCreation() = runBlocking {
        val source = DisplayActivitySource()
        val snapshot = source.update
        val channel = DisplayPowerRequests()
        val admission = channel.open(StreamingPower(), DisplayPowerPermit { source.current(snapshot) })
        val request = checkNotNull(channel.receive(checkNotNull(admission.notice)).request)
        val stamp = Any()
        val views = MutableStateFlow<PowerWorkerView?>(PowerWorkerView(request, MediaTask.Wake(true, true), stamp))
        val native = Native()
        var creates = 0
        val run = ScopedPowerRun(views, stamp, AtomicBoolean(), channel) { creates++; native }
        source.replace()
        val job = launch(start = CoroutineStart.UNDISPATCHED) { run.execute() }
        try {
            assertEquals("T1069 invalid original view admitted lazy factory", 0, creates)
            val fresh = source.update
            val notice = checkNotNull(channel.publish(admission.owner, StreamingPower(), DisplayPowerPermit { source.current(fresh) }))
            views.value = PowerWorkerView(checkNotNull(channel.receive(notice).request), MediaTask.Wake(true, true), stamp)
            yield()
            assertEquals(1, creates); assertEquals(1, native.executes)
        } finally { run.cancel(); job.cancelAndJoin(); run.retire() }
        assertEquals(1, native.retires)
    }
}
