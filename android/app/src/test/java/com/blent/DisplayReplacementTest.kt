package com.blent

import android.graphics.SurfaceTexture
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import java.lang.reflect.Proxy
import java.util.concurrent.CopyOnWriteArrayList

internal class DisplayReplacementRig {
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    private val texture = SurfaceTexture(0)
    private val surface = Surface(texture)
    val workers = CopyOnWriteArrayList<Run>()
    private val receivers = mutableListOf<RemoteDisplay>()
    class Run(val task: MediaTask.Display, val report: (WorkerEvent) -> Unit) : LifecycleRun {
        val retiring = CompletableDeferred<Unit>()
        val released = CompletableDeferred<Unit>()
        val retired = CompletableDeferred<Unit>()
        var cancellations = 0
        override suspend fun execute() = awaitCancellation()
        override fun cancel() { cancellations++ }
        override suspend fun retire() { retiring.complete(Unit); released.await(); retired.complete(Unit) }
    }
    private fun view() = object : SurfaceView(RuntimeEnvironment.getApplication()) {
        override fun getHolder(): SurfaceHolder = Proxy.newProxyInstance(SurfaceHolder::class.java.classLoader,
            arrayOf(SurfaceHolder::class.java)) { _, method, _ -> if (method.name == "getSurface") surface else null } as SurfaceHolder
    }
    fun receiver() = RemoteDisplay(RuntimeEnvironment.getApplication(), scope) { task, _, report ->
        Run(task, report).also { workers.add(it) }
    }.also { it.token = "a".repeat(64); it.setSurface(view()); receivers.add(it) }
    suspend fun close() {
        receivers.forEach { it.stop() }
        workers.forEach { it.released.complete(Unit) }
        workers.forEach { it.retired.await() }
        scope.coroutineContext[Job]!!.cancelAndJoin()
        surface.release(); texture.release()
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayReplacementTest {
    private suspend fun until(condition: () -> Boolean) { while (!condition()) delay(1) }

    @Test fun t1416ReplacementWaitsForOriginalRetirementAndSurvivesOldStop() = runBlocking {
        val rig = DisplayReplacementRig()
        val original = rig.receiver()
        val successor = rig.receiver()
        try {
            original.start()
            val first = rig.workers.single()
            var receipt: RenderAcknowledgement? = null
            original.onFrameRendered = { receipt = it }
            first.report(WorkerEvent.Rendered(1, 1, null))
            assertTrue(receipt!!.current())
            original.stop(); first.retiring.await()
            successor.start()
            assertEquals("T1416 replacement bypassed original native retirement", 1, rig.workers.size)
            first.released.complete(Unit)
            until { rig.workers.size == 2 }
            original.stop(); original.onSurfaceDestroyed()
            assertEquals(0, rig.workers.last().cancellations)
            assertFalse(receipt!!.current())
            first.report(WorkerEvent.Display(true, 99f, 99f, "obsolete", null, DecoderDiagnostics()))
            assertEquals(0f, successor.getFps())
            rig.workers.last().report(WorkerEvent.Display(true, 30f, 1f, null, null, DecoderDiagnostics()))
            assertEquals(30f, successor.getFps())
        } finally { rig.close() }
    }

    @Test fun t1416OnlyLatestWantedReplacementMayStart() = runBlocking {
        val rig = DisplayReplacementRig()
        val original = rig.receiver()
        val displaced = rig.receiver()
        val latest = rig.receiver()
        try {
            original.start()
            val first = rig.workers.single()
            original.stop(); first.retiring.await()
            displaced.start(); latest.start()
            latest.streamFps = 35
            displaced.stop(); displaced.onSurfaceDestroyed()
            assertEquals("T1416 pending display requests must stay bounded", 1, rig.workers.size)
            first.released.complete(Unit)
            until { rig.workers.size == 2 }
            assertEquals(35, rig.workers.last().task.format.fps)
        } finally { rig.close() }
    }

    @Test fun t1416StoppedPendingReplacementNeverAcquiresWorker() = runBlocking {
        val rig = DisplayReplacementRig()
        val original = rig.receiver()
        val successor = rig.receiver()
        try {
            original.start()
            val first = rig.workers.single()
            original.stop(); first.retiring.await()
            successor.start(); successor.stop()
            first.released.complete(Unit); first.retired.await(); yield()
            assertEquals("T1416 withdrawn replacement acquired a worker", 1, rig.workers.size)
        } finally { rig.close() }
    }

    @Test fun t1416UnstartedAndSupersededViewsCannotCancelCurrentDisplay() = runBlocking {
        val rig = DisplayReplacementRig()
        val original = rig.receiver()
        try {
            original.start()
            val first = rig.workers.single()
            var receipt: RenderAcknowledgement? = null
            original.onFrameRendered = { receipt = it }
            first.report(WorkerEvent.Rendered(1, 1, null))
            val successor = rig.receiver()
            successor.stop()
            assertEquals(0, first.cancellations)
            successor.start(); first.retiring.await()
            assertFalse("T1416 superseded client still authenticates old receipts", receipt!!.current())
            original.onSurfaceDestroyed(); original.stop()
            first.released.complete(Unit)
            until { rig.workers.size == 2 }
            assertEquals(0, rig.workers.last().cancellations)
        } finally { rig.close() }
    }
}
