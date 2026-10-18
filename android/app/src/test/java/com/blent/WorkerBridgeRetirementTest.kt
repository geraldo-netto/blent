package com.blent

import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class WorkerBridgeRetirementTest {
    private class NativeRun : LifecycleRun {
        val entered = CompletableDeferred<Job>()
        val retiring = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        var fail = false
        var live = false
        override suspend fun execute() { live = true; entered.complete(checkNotNull(currentCoroutineContext()[Job])); awaitCancellation() }
        override fun cancel() {}
        override suspend fun retire() {
            retiring.complete(Unit); release.await()
            check(!fail) { "T1065 original local cleanup remains unknown" }
            live = false
        }
    }
    private fun receiver() = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int) = true
    }
    private fun start(bridge: WorkerTestBridge, revision: Long) =
        WorkerIpc.command(bridge, WorkerIpc.START, revision, MediaTaskJson.encode(MediaTask.Metadata(1, 1, 1)), receiver())
    @Test fun t1065ExitCannotPublishDeathBeforeOriginalLocalJobAndRetirementSettle() = runBlocking {
        val entered = CompletableDeferred<Job>()
        val retiring = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val dead = CompletableDeferred<Unit>()
        var retired = 0
        val bridge = WorkerTestBridge { _, _, _ -> object : LifecycleRun {
            override suspend fun execute() { entered.complete(checkNotNull(currentCoroutineContext()[Job])); awaitCancellation() }
            override fun cancel() {}
            override suspend fun retire() { retiring.complete(Unit); release.await(); retired++ }
        } }
        val receiver = object : Binder() {
            override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int) = true
        }
        bridge.linkToDeath(IBinder.DeathRecipient { dead.complete(Unit) }, 0)
        WorkerIpc.command(bridge, WorkerIpc.START, 1, MediaTaskJson.encode(MediaTask.Metadata(1, 1, 1)), receiver)
        val original = withTimeout(2000) { entered.await() }
        try {
            WorkerIpc.command(bridge, WorkerIpc.EXIT, 1)
            withTimeout(2000) { retiring.await() }
            assertFalse("T1065 cancellation fabricated death before original native cleanup", dead.isCompleted)
            assertFalse(original.isCompleted)
            assertEquals(0, retired)
        } finally { release.complete(Unit); withContext(NonCancellable) { original.join() } }
        withTimeout(2000) { dead.await() }
        assertTrue(original.isCompleted); assertEquals(1, retired)
    }
    @Test fun t1065UnknownCleanupCannotBecomeDeathFromJobCompletionAlone() = runBlocking {
        val native = NativeRun().apply { fail = true }
        val bridge = WorkerTestBridge { _, _, _ -> native }
        var deaths = 0
        bridge.linkToDeath(IBinder.DeathRecipient { deaths++ }, 0)
        start(bridge, 1)
        val job = withTimeout(2000) { native.entered.await() }
        try {
            WorkerIpc.command(bridge, WorkerIpc.EXIT, 1)
            native.release.complete(Unit)
            withTimeout(2000) { job.join(); bridge.awaitExitAttempt(1) }
            assertTrue(job.isCompleted); assertTrue(native.live)
            assertEquals("T1065 failed native cleanup became simulated death", 0, deaths)
            assertTrue(runCatching { start(bridge, 2) }.isFailure)
        } finally { native.fail = false; native.release.complete(Unit); withContext(NonCancellable) { job.join(); native.retire() } }
        assertFalse(native.live)
    }
    @Test fun t1065OldGenerationAndOverlappingStartCannotCancelOrOverwriteOriginalExecution() = runBlocking {
        val first = NativeRun(); val next = NativeRun()
        val candidates = java.util.ArrayDeque(listOf(first, next))
        var creates = 0
        val bridge = WorkerTestBridge { _, _, _ -> creates++; candidates.removeFirst() }
        val firstDeath = CompletableDeferred<Unit>()
        val recipient = IBinder.DeathRecipient { firstDeath.complete(Unit) }
        bridge.linkToDeath(recipient, 0)
        start(bridge, 1)
        val original = withTimeout(2000) { first.entered.await() }
        try {
            assertTrue(runCatching { start(bridge, 2) }.isFailure)
            assertTrue(runCatching { WorkerIpc.command(bridge, WorkerIpc.STOP, 2) }.isFailure)
            assertEquals(1, creates); assertFalse(original.isCancelled)
            WorkerIpc.command(bridge, WorkerIpc.EXIT, 1)
        } finally { first.release.complete(Unit); withContext(NonCancellable) { original.join(); bridge.awaitExitAttempt(1) } }
        firstDeath.await(); assertTrue(bridge.unlinkToDeath(recipient, 0))
        start(bridge, 2)
        val replacement = withTimeout(2000) { next.entered.await() }
        try {
            assertTrue(runCatching { WorkerIpc.command(bridge, WorkerIpc.EXIT, 1) }.isFailure)
            assertFalse(replacement.isCancelled); assertEquals(2, creates)
            WorkerIpc.command(bridge, WorkerIpc.EXIT, 2)
        } finally { next.release.complete(Unit); withContext(NonCancellable) { replacement.join(); bridge.awaitExitAttempt(2) } }
        assertFalse(next.live)
    }
    @Test fun t1065InvalidEpochAndSurfaceMarkersNeverCreateAnExecution() {
        var creates = 0
        val bridge = WorkerTestBridge { _, _, _ -> creates++; error("T1065 invalid factory") }
        for (revision in listOf(Long.MIN_VALUE, -1L, 0L)) assertTrue(runCatching { start(bridge, revision) }.isFailure)
        for (marker in listOf(Int.MIN_VALUE, -1, 2, Int.MAX_VALUE)) {
            val data = Parcel.obtain()
            try {
                data.writeInterfaceToken(WorkerIpc.REQUEST); data.writeLong(1)
                data.writeString(MediaTaskJson.encode(MediaTask.Metadata(1, 1, 1))); data.writeStrongBinder(receiver()); data.writeInt(marker)
                assertFalse(bridge.transact(WorkerIpc.START, data, null, IBinder.FLAG_ONEWAY))
            } finally { data.recycle() }
        }
        assertEquals(0, creates)
        assertTrue(runCatching { WorkerIpc.command(bridge, 999, 1) }.isFailure)
    }
    @Test fun t1065LateCreatorAfterExitIsRetainedButNeverExecuted() = runBlocking {
        val entered = CompletableDeferred<Unit>()
        val creator = java.util.concurrent.CountDownLatch(1)
        val native = NativeRun().apply { release.complete(Unit) }
        val dead = CompletableDeferred<Unit>()
        val bridge = WorkerTestBridge { _, _, _ -> entered.complete(Unit); creator.await(); native }
        bridge.linkToDeath(IBinder.DeathRecipient { dead.complete(Unit) }, 0)
        start(bridge, 1)
        try {
            withTimeout(2000) { entered.await() }
            WorkerIpc.command(bridge, WorkerIpc.EXIT, 1)
            assertFalse(dead.isCompleted)
            assertFalse(native.entered.isCompleted)
        } finally {
            creator.countDown()
            withContext(NonCancellable) { WorkerIpc.command(bridge, WorkerIpc.EXIT, 1); bridge.awaitExitAttempt(1) }
        }
        withTimeout(2000) { dead.await() }
        assertFalse(native.entered.isCompleted)
        assertTrue(native.retiring.isCompleted)
        assertFalse(native.live)
    }
    @Test fun t1065ImmediateExitAlwaysSettlesAdmittedOrUnopenedOriginal() = runBlocking {
        repeat(32) {
            val native = NativeRun().apply { release.complete(Unit) }
            val bridge = WorkerTestBridge { _, _, _ -> native }
            val dead = CompletableDeferred<Unit>()
            bridge.linkToDeath(IBinder.DeathRecipient { dead.complete(Unit) }, 0)
            start(bridge, 1)
            WorkerIpc.command(bridge, WorkerIpc.EXIT, 1)
            withTimeout(2000) { bridge.awaitExitAttempt(1); dead.await() }
            assertFalse(native.live)
        }
    }
}
