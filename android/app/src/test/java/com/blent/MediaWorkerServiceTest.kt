package com.blent

import android.content.Intent
import android.graphics.SurfaceTexture
import android.os.*
import android.view.Surface
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger

class WorkerProbeService : MediaWorkerService() {
    internal var permitted = true
    internal var factory: (MediaTask, Surface?, (WorkerEvent) -> Unit) -> LifecycleRun = { _, _, _ -> error("fixture missing") }
    val terminations = AtomicInteger()
    override fun accepts(task: MediaTask) = permitted
    override fun createRun(task: MediaTask, surface: Surface?, report: (WorkerEvent) -> Unit) = factory(task, surface, report)
    override fun terminate() { terminations.incrementAndGet() }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class MediaWorkerServiceTest {
    private class Events : Binder() {
        val values = CopyOnWriteArrayList<WorkerEvent>()
        var reject = false
        var unlinkFails = false
        var death: IBinder.DeathRecipient? = null
        override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { death = recipient }
        override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int): Boolean { if (unlinkFails) error("unlink failed"); death = null; return true }
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            WorkerIpc.validate(data, WorkerIpc.EVENT)
            assertEquals(7L, data.readLong())
            values.add(WorkerEventJson.decode(requireNotNull(data.readString())))
            return !reject
        }
    }
    private fun request() = MediaTask.Metadata(640, 480, 30)
    private suspend fun until(condition: () -> Boolean) = withTimeout(2000) { while (!condition()) delay(1) }
    @Test fun t809_stopNeverWaitsForHeldRetirementAndExitIsGenerationScoped() = runBlocking {
        val controller = Robolectric.buildService(WorkerProbeService::class.java).create()
        val service = controller.get()
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val cancelled = AtomicInteger()
        val events = Events()
        service.factory = { _, _, report -> object : LifecycleRun {
            override suspend fun execute() { report(WorkerEvent.Status("ready")); entered.complete(Unit); awaitCancellation() }
            override fun cancel() { cancelled.incrementAndGet() }
            override suspend fun retire() { release.await() }
        } }
        val binder = service.onBind(Intent().putExtra("worker_generation", 7L))
        try {
            WorkerIpc.command(binder, WorkerIpc.START, 7, MediaTaskJson.encode(request()), events)
            entered.await()
            WorkerIpc.command(binder, WorkerIpc.STOP, 7); WorkerIpc.command(binder, WorkerIpc.STOP, 7)
            until { cancelled.get() > 0 }
            assertTrue(events.values.none { it is WorkerEvent.Ended })
            assertThrows(IllegalStateException::class.java) { WorkerIpc.command(binder, WorkerIpc.EXIT, 6) }
            assertEquals(0, service.terminations.get())
            WorkerIpc.command(binder, WorkerIpc.EXIT, 7)
            assertEquals(1, service.terminations.get())
            events.death!!.binderDied(); assertEquals(2, service.terminations.get())
            release.complete(Unit)
            until { events.values.any { it is WorkerEvent.Ended } }
            assertEquals(null, events.values.filterIsInstance<WorkerEvent.Ended>().single().failure)
            assertEquals(android.app.Service.START_NOT_STICKY, service.onStartCommand(null, 0, 0))
            assertFalse(service.onUnbind(Intent()))
        } finally { release.complete(Unit); controller.destroy() }
        assertNull(events.death)
    }
    @Test fun t809_failedOperationsAndCleanupAreReportedAndSurfaceLeaseReleased() = runBlocking {
        for (phase in listOf("create", "execute", "cancel", "retire")) {
            val controller = Robolectric.buildService(WorkerProbeService::class.java).create()
            val service = controller.get(); val events = Events()
            val surface = Surface(SurfaceTexture(0))
            val retired = AtomicInteger()
            service.factory = { _, borrowed, _ ->
                assertNotNull(borrowed)
                if (phase == "create") error("create")
                object : LifecycleRun {
                    override suspend fun execute() { if (phase == "execute") error("execute") }
                    override fun cancel() { if (phase == "cancel") error("cancel") }
                    override suspend fun retire() { retired.incrementAndGet(); if (phase == "retire") error("retire") }
                }
            }
            try {
                val binder = service.onBind(Intent().putExtra("worker_generation", 7L))
                WorkerIpc.command(binder, WorkerIpc.START, 7, MediaTaskJson.encode(request()), events, surface)
                until { events.values.any { it is WorkerEvent.Ended } }
                assertEquals(phase, events.values.filterIsInstance<WorkerEvent.Ended>().single().failure)
                if (phase != "create") assertEquals("T809 cancellation failure skipped retirement", 1, retired.get())
                assertTrue("Worker destroyed caller Surface", surface.isValid)
            } finally { controller.destroy(); surface.release() }
        }
    }
    @Test fun t809_malformedDuplicateAndRejectedCommandsCannotStartAnotherOwner() = runBlocking {
        for (mode in 0..3) {
            val controller = Robolectric.buildService(WorkerProbeService::class.java).create()
            val service = controller.get()
            val events = Events()
            val opened = AtomicInteger()
            service.permitted = mode != 1
            service.factory = { _, _, report ->
                opened.incrementAndGet()
                object : LifecycleRun {
                    override suspend fun execute() {
                        if (mode == 3) report(WorkerEvent.Result("x".repeat(WorkerEventJson.LIMIT + 1)))
                    }
                    override fun cancel() {}
                    override suspend fun retire() {}
                }
            }
            try {
                assertThrows(IllegalArgumentException::class.java) { service.onBind(Intent()) }
                val binder = service.onBind(Intent().putExtra("worker_generation", 7L))
                if (mode == 0) assertThrows(IllegalStateException::class.java) { WorkerIpc.command(binder, WorkerIpc.START, 7, "bad", events) }
                else if (mode == 1) assertThrows(IllegalStateException::class.java) { WorkerIpc.command(binder, WorkerIpc.START, 7, MediaTaskJson.encode(request()), events) }
                else {
                    events.reject = mode == 2
                    WorkerIpc.command(binder, WorkerIpc.START, 7, MediaTaskJson.encode(request()), events)
                    until { service.terminations.get() > 0 }
                    assertThrows(IllegalStateException::class.java) { WorkerIpc.command(binder, WorkerIpc.START, 7, MediaTaskJson.encode(request()), events) }
                    assertEquals(1, opened.get())
                }
                assertTrue(service.terminations.get() > 0)
                assertThrows(IllegalStateException::class.java) { WorkerIpc.command(binder, 99, 7) }
            } finally { controller.destroy() }
        }
    }
    @Test fun t809_destroyStillTerminatesWhenDeathUnlinkFails() = runBlocking {
        val controller = Robolectric.buildService(WorkerProbeService::class.java).create()
        val service = controller.get()
        val events = Events()
        val binder = service.onBind(Intent().putExtra("worker_generation", 7L))
        WorkerIpc.command(binder, WorkerIpc.START, 7, MediaTaskJson.encode(request()), events)
        until { events.values.any { it is WorkerEvent.Ended } }
        events.unlinkFails = true
        val before = service.terminations.get()
        try { controller.destroy() } catch (_: Exception) {}
        assertTrue("T809 failed unlink bypasses process cleanup", service.terminations.get() > before)
    }
    @Test fun t809_nativeFactoryAndServiceTypeAdmissionRemainSeparate() = runBlocking {
        val controller = Robolectric.buildService(MetadataWorkerService::class.java).create()
        try {
            val service = controller.get()
            val events = mutableListOf<WorkerEvent>()
            val run = service.createRun(request(), null, events::add)
            run.execute(); run.cancel(); run.retire()
            assertTrue(events.single() is WorkerEvent.Result)
            val token = "a".repeat(64)
            val tasks = listOf<MediaTask>(request(), MediaTask.Display(AdbDisplayRoutes.create(token), DecoderFormat("video/avc", 1, 1, 30)),
                MediaTask.Camera(CameraEndpoint(token, 1, 640, 480, 30, 1000), CameraLens.REAR, 0),
                MediaTask.Audio(AudioEndpoint(token, 1, 1, 1, 40, false), AudioPreferences()),
                MediaTask.Audio(AudioEndpoint(token, 1, 2, 1, 40, false), AudioPreferences()))
            val services = listOf(service, DisplayWorkerService(), CameraWorkerService(), MicrophoneWorkerService(), SpeakerWorkerService())
            for ((index, owner) in services.withIndex()) for ((other, task) in tasks.withIndex()) assertEquals(index == other, owner.accepts(task))
        } finally { controller.destroy() }
    }
}
