package com.blent

import android.content.Context
import android.view.Surface
import kotlinx.coroutines.*

/** Native ownership factory, used exclusively inside a private worker process. */
internal object LocalMediaRuns {
    fun create(context: Context, task: MediaTask, surface: Surface?, report: (WorkerEvent) -> Unit): LifecycleRun = when (task) {
        is MediaTask.Audio -> audio(context, task, report)
        is MediaTask.Camera -> camera(context, task, report)
        is MediaTask.Display -> display(task, requireNotNull(surface), report)
        is MediaTask.Metadata -> metadata(task, report)
        is MediaTask.Wake -> PowerWakeRun(task, { AndroidWakeHandles.cpu(context) }, { AndroidWakeHandles.wifi(context) }) {
            report(WorkerEvent.WakeReady(task.cpu, task.wifi))
        }
    }
    private fun audio(context: Context, task: MediaTask.Audio, report: (WorkerEvent) -> Unit): LifecycleRun {
        val resources = AudioResources()
        val capture = if (task.endpoint.direction == 1) AudioCapture(context)::run else AudioPlayback(context)::run
        return object : LifecycleRun {
            override suspend fun execute() = capture(task.endpoint, task.preferences, resources::attach) { report(WorkerEvent.Status(it)) }
            override fun cancel() = resources.cancel()
            override suspend fun retire() = resources.cancel()
        }
    }
    private fun camera(context: Context, task: MediaTask.Camera, report: (WorkerEvent) -> Unit): LifecycleRun {
        val resources = CameraResources()
        return object : LifecycleRun {
            override suspend fun execute() = coroutineScope {
                launch { resources.startupStatus.collect { if (it != null) report(WorkerEvent.Status(it)) } }
                CameraCapture(context).run(task.endpoint, task.lens, task.rotation, resources)
            }
            override fun cancel() = resources.cancel()
            override suspend fun retire() = resources.close()
        }
    }
    private fun metadata(task: MediaTask.Metadata, report: (WorkerEvent) -> Unit) = object : LifecycleRun {
        override suspend fun execute() {
            val text = if (task.shellVersion != 0) MetadataQueries.lane.query { decoderReport(task.shellVersion) } ?: formatDecoderReport(task.shellVersion, null)
                else DecoderCapabilities.report(task.width, task.height, task.fps).toString()
            report(WorkerEvent.Result(text))
        }
        override fun cancel() {}
        override suspend fun retire() {}
    }
    private fun display(task: MediaTask.Display, surface: Surface, report: (WorkerEvent) -> Unit): LifecycleRun {
        val receiver = VideoReceiver(initialRoutes = task.routes)
        receiver.setVideoLiveness(task.liveness)
        receiver.setStreamFormat(task.format)
        receiver.setNativeSurface(surface)
        return DisplayWorkerRun(receiver, report)
    }
}

internal class DisplayWorkerRun(private val receiver: VideoReceiver, private val report: (WorkerEvent) -> Unit) : LifecycleRun {
    @Volatile private var connected = false
    private val stopped = CompletableDeferred<Unit>()
    override suspend fun execute() = coroutineScope {
        if (stopped.isCompleted) return@coroutineScope
        receiver.observeConnection({ connected = true; publish() }, { connected = false; publish() })
        receiver.onFrameRendered = { if (it.current()) report(WorkerEvent.Rendered(it.sequence, it.decodeMicros, it.decoder)) }
        receiver.start()
        try { while (!stopped.isCompleted) { publish(); withTimeoutOrNull(1000) { stopped.await() } } }
        finally { receiver.stop() }
    }
    private fun publish() = report(WorkerEvent.Display(connected, receiver.getFps(), receiver.getMbps(),
        CodecLifetime.startupStatus.value, CodecLifetime.retirementStatus.value, receiver.decoder.diagnostics.value))
    override fun cancel() { stopped.complete(Unit); receiver.stop() }
    override suspend fun retire() {
        cancel()
        while (CodecLifetime.retirementPending() || CodecLifetime.startupStatus.value != null) delay(10)
    }
}
