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

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class RemoteDisplayTest {
    private fun view(surface: Surface) = object : SurfaceView(RuntimeEnvironment.getApplication()) {
        override fun getHolder(): SurfaceHolder = Proxy.newProxyInstance(SurfaceHolder::class.java.classLoader,
            arrayOf(SurfaceHolder::class.java)) { _, method, _ -> if (method.name == "getSurface") surface else null } as SurfaceHolder
    }
    @Test fun t856DefaultPresentationSelectsTheAdbAdapterWithoutStartingMedia() {
        val receiver = RemoteDisplay(RuntimeEnvironment.getApplication())
        assertEquals(AdbDisplayRoutes.create(), receiver.routes)
        assertNull(receiver.token)
        receiver.stop()
    }
    @Test fun t856RouteBundleCrossesWorkerBoundaryAndRetiresOldReceipts() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val requests = mutableListOf<MediaTask.Display>()
        val callbacks = mutableListOf<(WorkerEvent) -> Unit>()
        val retirement = CompletableDeferred<Unit>()
        val surface = Surface(SurfaceTexture(0))
        val receiver = RemoteDisplay(RuntimeEnvironment.getApplication(), scope) { task, _, report ->
            requests.add(task); callbacks.add(report)
            object : LifecycleRun {
                override suspend fun execute() = awaitCancellation()
                override fun cancel() {}
                override suspend fun retire() { retirement.await() }
            }
        }
        try {
            val selected = DisplayRoutes(2, "a".repeat(64), 12001, 12002)
            receiver.routes = selected; receiver.setSurface(view(surface)); receiver.start()
            assertEquals(selected, requests.single().routes); assertEquals(selected, receiver.routes)
            assertEquals(selected.token, receiver.token)
            receiver.routes = selected; assertEquals(1, requests.size)
            val receipts = mutableListOf<RenderAcknowledgement>()
            receiver.onFrameRendered = { receipts.add(it) }
            callbacks[0](WorkerEvent.Rendered(1, 2, null)); assertTrue(receipts.single().current())
            receiver.routes = selected.copy(revision = 3, videoPort = 13001, controlPort = 13002)
            assertFalse(receipts.single().current()); assertEquals(1, requests.size)
            assertThrows(IllegalArgumentException::class.java) { receiver.routes = selected }
            receiver.routes = null; receiver.routes = null; receiver.token = selected.token
            assertNull(receiver.routes); assertNull(receiver.token)
            retirement.complete(Unit)
            val resumed = selected.copy(revision = 4)
            receiver.routes = resumed
            withTimeout(2000) { while (requests.size < 2) delay(1) }
            assertEquals(resumed, requests.last().routes)
            callbacks[0](WorkerEvent.Rendered(2, 3, null)); assertEquals(1, receipts.size)
        } finally { retirement.complete(Unit); receiver.stop(); scope.cancel(); surface.release() }
    }
    @Test fun t809_surfaceFormatStopAndStaleFramesKeepOneOwnedGeneration() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val callbacks = mutableListOf<(WorkerEvent) -> Unit>()
        val requests = mutableListOf<MediaTask.Display>()
        val retirement = CompletableDeferred<Unit>()
        val surface = Surface(SurfaceTexture(0))
        val receiver = RemoteDisplay(RuntimeEnvironment.getApplication(), scope) { task, _, report ->
            requests.add(task); callbacks.add(report)
            object : LifecycleRun {
                override suspend fun execute() = awaitCancellation()
                override fun cancel() {}
                override suspend fun retire() { retirement.await() }
            }
        }
        try {
            var connected = 0; var disconnected = 0
            receiver.observeConnection({ connected++ }, { disconnected++ })
            receiver.start(); assertTrue(requests.isEmpty())
            receiver.setSurface(view(surface)); assertTrue(requests.isEmpty())
            receiver.token = "a".repeat(64)
            receiver.stop(); receiver.start(); receiver.start()
            assertEquals(1, requests.size)
            val sameView = view(surface)
            receiver.setSurface(sameView); assertEquals(1, requests.size)
            val receipts = mutableListOf<RenderAcknowledgement>()
            receiver.onFrameRendered = { receipts.add(it) }
            callbacks[0](WorkerEvent.Rendered(7, 50, "fixture"))
            assertTrue(receipts.single().current())
            callbacks[0](WorkerEvent.Display(true, 30f, 2f, "startup", "retire", DecoderDiagnostics(watchdogFallback = true)))
            assertEquals(1, connected); assertEquals(30f, receiver.getFps()); assertEquals(2f, receiver.getMbps())
            receiver.observeConnection({ connected++ }, { disconnected++ }); assertEquals(2, connected)
            receiver.streamFps = 999; assertEquals(60, receiver.streamFps)
            receiver.streamFps = 30
            assertFalse(receipts.single().current())
            repeat(100) { receiver.setStreamFormat(DecoderFormat("video/avc", 640 + it, 480, 30)) }
            assertEquals("T809 replacements bypass held retirement", 1, requests.size)
            callbacks[0](WorkerEvent.Rendered(8, 50, null)); assertEquals(1, receipts.size)
            callbacks[0](WorkerEvent.Display(true, 100f, 100f, null, null, DecoderDiagnostics()))
            assertEquals(0f, receiver.getFps())
            retirement.complete(Unit)
            withTimeout(1000) { while (requests.size != 2) delay(1) }
            assertEquals(739, requests.last().format.width)
            callbacks.last()(WorkerEvent.Status("ignored"))
            callbacks.last()(WorkerEvent.Display(false, 1f, 1f, null, null, DecoderDiagnostics()))
            receiver.mimeType = "video/hevc"; assertEquals("video/hevc", receiver.mimeType)
            receiver.onSurfaceDestroyed()
            val count = requests.size
            receiver.stop(); receiver.start(); assertEquals(count, requests.size)
            receiver.stop()
            assertTrue(disconnected > 0)
        } finally { retirement.complete(Unit); receiver.stop(); scope.cancel(); surface.release() }
    }
    @Test fun t809_workerFailureAndFailedRetirementRemainVisible() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val surface = Surface(SurfaceTexture(0))
        val receiver = RemoteDisplay(RuntimeEnvironment.getApplication(), scope) { _, _, _ -> object : LifecycleRun {
            override suspend fun execute() { error("worker failed") }
            override fun cancel() {}
            override suspend fun retire() { error("exit unconfirmed") }
        } }
        try {
            receiver.token = "a".repeat(64); receiver.setSurface(view(surface)); receiver.start()
            assertEquals("worker failed", receiver.startupStatus.value)
            assertTrue(receiver.retirementStatus.value!!.contains("exit unconfirmed"))
            receiver.stop(); receiver.start()
            assertTrue(receiver.startupStatus.value!!.contains("exit unconfirmed"))
        } finally { receiver.stop(); scope.cancel(); surface.release() }
    }
    @Test fun t809_completedWorkerInvalidatesReceiptsAndAllowsExplicitRestart() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val surface = Surface(SurfaceTexture(0))
        val done = CompletableDeferred<Unit>()
        val callbacks = mutableListOf<(WorkerEvent) -> Unit>()
        val receiver = RemoteDisplay(RuntimeEnvironment.getApplication(), scope) { _, _, report ->
            callbacks.add(report)
            object : LifecycleRun {
                override suspend fun execute() { done.await() }
                override fun cancel() {}
                override suspend fun retire() {}
            }
        }
        try {
            var receipt: RenderAcknowledgement? = null
            receiver.onFrameRendered = { receipt = it }
            receiver.token = "a".repeat(64); receiver.setSurface(view(surface)); receiver.start()
            callbacks.single()(WorkerEvent.Rendered(1, 1, null))
            assertTrue(receipt!!.current())
            done.complete(Unit)
            assertFalse("T809 dead worker receipt remains current", receipt!!.current())
            callbacks.single()(WorkerEvent.Display(true, 30f, 1f, null, null, DecoderDiagnostics()))
            assertEquals(0f, receiver.getFps())
            receiver.start()
            assertEquals("T809 explicit restart ignored after worker exit", 2, callbacks.size)
        } finally { receiver.stop(); scope.cancel(); surface.release() }
    }
    @Test fun t809_defaultConnectionCallbackAndFailedStatusAreContained() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val surface = Surface(SurfaceTexture(0))
        lateinit var publish: (WorkerEvent) -> Unit
        val receiver = RemoteDisplay(RuntimeEnvironment.getApplication(), scope) { _, _, events ->
            publish = events
            object : LifecycleRun {
                override suspend fun execute() = awaitCancellation()
                override fun cancel() {}
                override suspend fun retire() {}
            }
        }
        try {
            receiver.token = "a".repeat(64); receiver.setSurface(view(surface)); receiver.start()
            publish(WorkerEvent.Display(true, 30f, 1f, null, null, DecoderDiagnostics()))
            assertEquals(30f, receiver.getFps())
            receiver.observeConnection({}, { error("callback failed") })
            publish(WorkerEvent.Display(false, 0f, 0f, null, null, DecoderDiagnostics()))
            assertEquals("Display status update failed: callback failed", receiver.startupStatus.value)
            receiver.observeConnection({}, {})
        } finally { receiver.stop(); scope.cancel(); surface.release() }
    }
    @Test fun t809_localDisplayAdapterStopsNormallyAndNeverStartsAfterCancel() = runBlocking {
        val receiver = VideoReceiver { error("No native socket expected without Surface") }
        val events = mutableListOf<WorkerEvent>()
        val run = DisplayWorkerRun(receiver, events::add)
        val job = launch(start = CoroutineStart.UNDISPATCHED) { run.execute() }
        assertTrue(events.any { it is WorkerEvent.Display })
        receiver.onConnected?.invoke()
        receiver.onDisconnected?.invoke()
        receiver.onFrameRendered?.invoke(RenderAcknowledgement(1, 5))
        receiver.onFrameRendered?.invoke(RenderAcknowledgement(2, 5) { false })
        assertEquals(1, events.filterIsInstance<WorkerEvent.Rendered>().size)
        run.cancel(); withTimeout(1000) { job.join() }; run.retire()
        val stopped = DisplayWorkerRun(VideoReceiver(), { error("Cancelled worker restarted") })
        stopped.cancel(); stopped.execute(); stopped.retire()
        val surface = Surface(SurfaceTexture(0))
        try {
            val native = LocalMediaRuns.create(RuntimeEnvironment.getApplication(), MediaTask.Display(AdbDisplayRoutes.create("a".repeat(64)), DecoderFormat("video/avc", 640, 480, 30)), surface, {})
            native.cancel(); native.retire()
        } finally { surface.release() }
    }
}
