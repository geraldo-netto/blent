package com.blent

import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class ScopedPowerRunTest {
    private class Run : LifecycleRun {
        var executes = 0
        var canceled = 0
        var retired = 0
        override suspend fun execute() { executes++; awaitCancellation() }
        override fun cancel() { canceled++ }
        override suspend fun retire() { retired++ }
    }
    @Test fun t1060UnacknowledgedViewWaitsForActualCurrentReceiptWithoutNativeAdmission() = runBlocking {
        val channel = DisplayPowerRequests()
        val original = channel.open(StreamingPower())
        val request = checkNotNull(channel.receive(checkNotNull(original.notice)).request)
        val stamp = Any()
        val views = MutableStateFlow<PowerWorkerView?>(PowerWorkerView(request, MediaTask.Wake(true, true), stamp))
        val notice = checkNotNull(channel.publish(original.owner, StreamingPower(true)))
        val native = Run()
        var created = 0
        val run = ScopedPowerRun(views, stamp, AtomicBoolean(), channel) { created++; native }
        val job = launch(start = CoroutineStart.UNDISPATCHED) { run.execute() }
        try {
            assertEquals(0, created)
            val current = checkNotNull(channel.receive(notice).request)
            views.value = PowerWorkerView(current, MediaTask.Wake(true, true), stamp)
            yield()
            assertEquals(1, created); assertEquals(1, native.executes)
            assertTrue(runCatching { run.execute() }.isFailure)
        } finally { run.cancel(); job.cancelAndJoin(); run.retire() }
        assertEquals(1, native.retired)
    }
    @Test fun t1060CutoffForeignStampAndUnopenedRetirementNeverCreateNativeWorker() = runBlocking {
        val channel = DisplayPowerRequests()
        val original = channel.open(StreamingPower())
        val request = checkNotNull(channel.receive(checkNotNull(original.notice)).request)
        val views = MutableStateFlow<PowerWorkerView?>(PowerWorkerView(request, MediaTask.Wake(true, true), Any()))
        val foreign = ScopedPowerRun(views, Any(), AtomicBoolean(), channel) { error("T1060 wrong stamp factory") }
        foreign.execute(); foreign.retire()
        val cutoff = ScopedPowerRun(views, checkNotNull(views.value).stamp, AtomicBoolean(true), channel) { error("T1060 cut off factory") }
        cutoff.execute(); cutoff.retire()
        val unopened = ScopedPowerRun(views, checkNotNull(views.value).stamp, AtomicBoolean(), channel) { error("T1060 retired factory") }
        unopened.retire(); unopened.execute()
        views.value = null
        ScopedPowerRun(views, Any(), AtomicBoolean(), channel) { error("T1060 closed view factory") }.execute()
    }
    @Test fun t1060LateLazyCreatorRemainsOwnedAndJoinedBeforeOriginalRetirement() = runBlocking {
        val channel = DisplayPowerRequests()
        val original = channel.open(StreamingPower())
        val request = checkNotNull(channel.receive(checkNotNull(original.notice)).request)
        val stamp = Any()
        val views = MutableStateFlow<PowerWorkerView?>(PowerWorkerView(request, MediaTask.Wake(true, true), stamp))
        val entered = CompletableDeferred<Unit>()
        val release = CountDownLatch(1)
        val native = Run()
        val run = ScopedPowerRun(views, stamp, AtomicBoolean(), channel) { entered.complete(Unit); release.await(); native }
        val job = launch(Dispatchers.Default) { run.execute() }
        try {
            withTimeout(2000) { entered.await() }
            run.cancel(); job.cancel()
            assertFalse(job.isCompleted)
            assertEquals(0, native.executes); assertEquals(0, native.retired)
        } finally { release.countDown(); withContext(NonCancellable) { job.join(); run.retire() } }
        assertEquals(0, native.executes)
        assertTrue(native.canceled > 0); assertEquals(1, native.retired)
    }
}
