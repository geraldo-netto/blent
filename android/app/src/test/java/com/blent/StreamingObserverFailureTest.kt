package com.blent

import android.os.Looper
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class StreamingObserverFailureTest {
    private fun idle() = shadowOf(Looper.getMainLooper()).idle()
    private fun observer(binding: StreamingPowerBinding) =
        checkNotNull(ReflectionHelpers.getField<CoroutineScope>(binding, "scope").coroutineContext[Job])
    private class Exceptions : AutoCloseable {
        private val thread = Thread.currentThread()
        private val previous = thread.uncaughtExceptionHandler
        val escaped = CopyOnWriteArrayList<Throwable>()
        init { thread.uncaughtExceptionHandler = Thread.UncaughtExceptionHandler { _, error -> escaped.add(error) } }
        override fun close() { thread.uncaughtExceptionHandler = previous }
    }
    @Test fun t1073CanceledPolicyInputWithdrawsItsClaimWithoutAnUncaughtFailure() = runBlocking {
        Exceptions().use { errors ->
            val rig = PowerRuntimeFixture()
            val release = CompletableDeferred<Unit>()
            val binding = StreamingPowerBinding(rig.app, rig.runtime)
            binding.start(DisplayPowerDemand(StreamingPower()), flow { release.await(); throw CancellationException("T1073 source canceled") })
            idle()
            val owner = checkNotNull(PowerServiceNotice.read(rig.app.starts.single())).notice.original.owner
            try {
                rig.attachFirst(); release.complete(Unit); idle(); rig.drain()
                assertTrue(errors.escaped.isEmpty())
                assertFalse("T1073 canceled source retained its foreground claim", rig.runtime.owns(owner))
                assertEquals(1, rig.settled)
            } finally { release.complete(Unit); binding.stop(); rig.finish(owner) }
        }
    }
    @Test fun t1073CleanupAndReporterFaultsCannotEscapeOrDiscardOriginalFailure() = runBlocking {
        Exceptions().use { errors ->
            val rig = PowerRuntimeFixture()
            val source = IllegalStateException("T1073 source failed")
            val cleanup = IllegalStateException("T1073 stop failed")
            val reporter = IllegalStateException("T1073 reporter failed")
            val release = CompletableDeferred<Unit>()
            val binding = StreamingPowerBinding(rig.app, rig.runtime) { throw reporter }
            binding.start(DisplayPowerDemand(StreamingPower()), flow { release.await(); throw source })
            idle()
            val owner = checkNotNull(PowerServiceNotice.read(rig.app.starts.single())).notice.original.owner
            try {
                rig.attachFirst(); rig.stopFailure = cleanup
                release.complete(Unit); idle()
                assertTrue(errors.escaped.isEmpty()); assertFalse(rig.runtime.owns(owner))
                assertTrue(source.suppressed.contains(cleanup)); assertTrue(source.suppressed.contains(reporter))
                assertEquals(1, rig.settled)
            } finally { rig.stopFailure = null; release.complete(Unit); binding.stop(); rig.finish(owner) }
        }
    }
    @Test fun t1073CanceledInputStillReportsIndependentStopFailure() = runBlocking {
        Exceptions().use { errors ->
            val rig = PowerRuntimeFixture()
            val cleanup = IllegalStateException("T1073 canceled source stop failed")
            val release = CompletableDeferred<Unit>()
            val binding = StreamingPowerBinding(rig.app, rig.runtime)
            binding.start(DisplayPowerDemand(StreamingPower()), flow { release.await(); throw CancellationException("T1073 canceled input") })
            idle()
            val owner = checkNotNull(PowerServiceNotice.read(rig.app.starts.single())).notice.original.owner
            try {
                rig.attachFirst(); rig.stopFailure = cleanup
                release.complete(Unit); idle()
                assertTrue(errors.escaped.isEmpty()); assertFalse(rig.runtime.owns(owner))
                assertTrue(org.robolectric.shadows.ShadowLog.getLogsForTag("BlentPower").any { it.throwable === cleanup })
            } finally { rig.stopFailure = null; release.complete(Unit); binding.stop(); rig.finish(owner) }
        }
    }
    @Test fun t1073TheSameStopAndReporterFailureCannotSuppressItself() = runBlocking {
        Exceptions().use { errors ->
            val rig = PowerRuntimeFixture()
            val failure = IllegalStateException("T1073 shared failure")
            val release = CompletableDeferred<Unit>()
            val binding = StreamingPowerBinding(rig.app, rig.runtime) { throw failure }
            binding.start(DisplayPowerDemand(StreamingPower()), flow { release.await(); throw failure })
            idle()
            val owner = checkNotNull(PowerServiceNotice.read(rig.app.starts.single())).notice.original.owner
            try {
                rig.attachFirst(); rig.stopFailure = failure
                release.complete(Unit); idle()
                assertTrue(errors.escaped.isEmpty()); assertFalse(rig.runtime.owns(owner))
                assertTrue(failure.suppressed.isEmpty())
            } finally { rig.stopFailure = null; release.complete(Unit); binding.stop(); rig.finish(owner) }
        }
    }
    @Test fun t1073PublisherFailureCannotEscapeItsObserverOrKeepObserving() = runBlocking {
        Exceptions().use { errors ->
            val rig = PowerRuntimeFixture()
            val failure = IllegalStateException("T1073 publisher stop failed")
            val reports = ArrayList<Exception>()
            val updates = MutableStateFlow(DisplayPowerDemand(StreamingPower()))
            val binding = StreamingPowerBinding(rig.app, rig.runtime, reports::add)
            binding.start(updates.value, updates); idle()
            val job = observer(binding)
            val owner = checkNotNull(PowerServiceNotice.read(rig.app.starts.single())).notice.original.owner
            try {
                rig.attachFirst(); rig.postRefused = true; rig.stopFailure = failure
                updates.value = DisplayPowerDemand(StreamingPower(true, true)); idle()
                assertTrue(errors.escaped.isEmpty()); assertFalse(rig.runtime.owns(owner))
                assertTrue(job.isCompleted); assertEquals(listOf(failure), reports)
                assertEquals(1, rig.settled)
                val count = rig.app.starts.size
                updates.value = DisplayPowerDemand(StreamingPower()); idle()
                assertEquals(count, rig.app.starts.size)
            } finally { rig.postRefused = false; rig.stopFailure = null; binding.stop(); rig.finish(owner) }
        }
    }
    @Test fun t1073FailedCurrentPolicyClosesOnlyItsOriginalProducerAndSettlesObserver() = runBlocking {
        Exceptions().use { errors ->
            val rig = PowerRuntimeFixture()
            val failure = IllegalStateException("T1073 current policy stream failed")
            val reports = ArrayList<Exception>()
            val release = CompletableDeferred<Unit>()
            val binding = StreamingPowerBinding(rig.app, rig.runtime, reports::add)
            binding.start(DisplayPowerDemand(StreamingPower()), flow { release.await(); throw failure })
            idle()
            val job = observer(binding)
            val owner = checkNotNull(PowerServiceNotice.read(rig.app.starts.single())).notice.original.owner
            try {
                rig.attachFirst()
                release.complete(Unit); idle(); rig.drain()
                assertTrue("T1073 ordinary failure escaped its resource owner: ${errors.escaped}", errors.escaped.isEmpty())
                assertFalse(rig.runtime.owns(owner))
                assertEquals(listOf(failure), reports)
                assertTrue(job.isCompleted)
                assertEquals(1, rig.settled)
            } finally { release.complete(Unit); binding.stop(); rig.finish(owner) }
        }
    }
    @Test fun t1073LateOldObserverFailureCannotCancelAReplacementProducer() = runBlocking {
        Exceptions().use { errors ->
            val rig = PowerRuntimeFixture()
            val failure = IllegalStateException("T1073 late old policy failure")
            val release = CompletableDeferred<Unit>()
            val reports = ArrayList<Exception>()
            val binding = StreamingPowerBinding(rig.app, rig.runtime, reports::add)
            binding.start(DisplayPowerDemand(StreamingPower()), flow { withContext(NonCancellable) { release.await(); throw failure } })
            idle()
            val old = observer(binding)
            val owner = checkNotNull(PowerServiceNotice.read(rig.app.starts.single())).notice.original.owner
            try {
                rig.attachFirst(); binding.stop()
                assertFalse(rig.runtime.owns(owner))
                val current = MutableStateFlow(DisplayPowerDemand(StreamingPower(true, true)))
                binding.start(current.value, current); idle(); rig.drain()
                assertEquals(2, rig.native.size)
                release.complete(Unit); idle(); rig.drain()
                assertTrue(errors.escaped.isEmpty()); assertTrue(old.isCompleted)
                assertTrue("T1073 late old error canceled replacement", rig.runtime.hasOwner)
                current.value = DisplayPowerDemand(StreamingPower()); idle(); rig.drain()
                assertEquals(3, rig.native.size)
                val reported = reports.single()
                // withContext may recover the normal exception's stack in a causal copy.
                assertTrue("T1073 original late failure lost", reported === failure || reported.cause === failure)
            } finally { release.complete(Unit); binding.stop(); rig.finish(owner) }
        }
    }
}
