package com.blent

import android.content.*
import android.graphics.SurfaceTexture
import android.os.*
import android.view.Surface
import kotlinx.coroutines.*
import org.json.*
import java.io.File
import java.util.concurrent.atomic.*

class WorkerAcceptanceReceiver : BroadcastReceiver() {
    companion object { private val active = AtomicBoolean() }
    override fun onReceive(context: Context, intent: Intent) {
        if (!active.compareAndSet(false, true)) return
        val pending = goAsync()
        Thread {
            try {
                val result = try { runBlocking { exercise(context.applicationContext, intent.getBooleanExtra("owner_death", false)) } }
                    catch (error: Throwable) { JSONObject().put("error", error.stackTraceToString()) }
                result.put("run_id", intent.getStringExtra("run_id"))
                File(context.filesDir, "workers.json").writeText(result.toString(2))
            } finally { active.set(false); pending.finish() }
        }.start()
    }
    private fun audio(direction: Int, mode: Int) = MediaTask.Audio(
        AudioEndpoint("a".repeat(64), 12345, direction, 1, mode, false), AudioPreferences(gain = 0, builtIn = false))
    @OptIn(ExperimentalCoroutinesApi::class)
    private suspend fun exercise(context: Context, ownerDeath: Boolean): JSONObject = supervisorScope {
        val rows = JSONArray()
        val owners = ArrayList<Owner>()
        val texture = SurfaceTexture(false)
        val surface = Surface(texture)
        try {
            val tasks = listOf(audio(1, 60), audio(2, 40),
                MediaTask.Camera(CameraEndpoint("a".repeat(64), 12345, 640, 480, 30, 3000), CameraLens.REAR, 0),
                MediaTask.Display(AdbDisplayRoutes.create("a".repeat(64)), DecoderFormat("video/avc", 640, 480, 30)))
            for (task in tasks) {
                val started = System.nanoTime()
                val owner = Owner(context, task, if (task is MediaTask.Display) surface else null, this).also { owners.add(it) }
                val pid = withTimeout(5000) { owner.pid.await() }
                check(pid != Process.myPid())
                rows.put(JSONObject().put("pid", pid).put("startup_ns", System.nanoTime() - started))
            }
            check(owners.map { it.pid.getCompleted() }.toSet().size == 4)
            if (ownerDeath) {
                File(context.filesDir, "owner-death.json").writeText(JSONObject().put("workers", rows).put("parent_pid", Process.myPid()).toString())
                Process.killProcess(Process.myPid())
                awaitCancellation()
            }
            val before = owners[1].heartbeats.get()
            val stopAt = System.nanoTime()
            owners[0].stop()
            val stopNs = System.nanoTime() - stopAt
            check(stopNs < 4_000_000_000L)
            check(owners[1].heartbeats.get() > before)
            check(surface.isValid)
            val restarted = Owner(context, audio(1, 40), null, this).also { owners.add(it) }
            check(withTimeout(5000) { restarted.pid.await() } != owners[0].pid.getCompleted())
            restarted.stop()
            val cleanup = Owner(context, audio(1, 80), null, this).also { owners.add(it) }
            withTimeout(5000) { cleanup.pid.await() }
            cleanup.stop()
            val dying = Owner(context, audio(1, 100), null, this).also { owners.add(it) }
            withTimeout(5000) { dying.pid.await() }
            val failure = withTimeout(5000) { dying.result.await() }
            check(failure != null)
            dying.stop()
            var metadata: String? = null
            val query = AndroidWorkerRun(context, MediaTask.Metadata(640, 480, 30), report = { if (it is WorkerEvent.Result) metadata = it.text })
            try { withTimeout(5000) { query.execute() } } finally { withContext(NonCancellable) { query.retire() } }
            check(JSONObject(checkNotNull(metadata)).getInt("width") == 640)
            JSONObject().put("fingerprint", Build.FINGERPRINT).put("workers", rows)
                .put("held_owner_stop_ns", stopNs).put("sibling_heartbeats", owners[1].heartbeats.get())
                .put("cleanup_hang_contained", true).put("worker_death_observed", true).put("native_metadata", JSONObject(metadata!!))
        } finally {
            for (owner in owners) withContext(NonCancellable) { owner.stop() }
            check(surface.isValid)
            surface.release(); texture.release()
        }
    }
    private class Owner(context: Context, task: MediaTask, surface: Surface?, scope: CoroutineScope) {
        val pid = CompletableDeferred<Int>()
        val heartbeats = AtomicInteger()
        private val run = AndroidWorkerRun(context, task, surface, { event ->
            if (event is WorkerEvent.Status) {
                if (event.text.startsWith("pid=")) pid.complete(event.text.removePrefix("pid=").toInt())
                else heartbeats.incrementAndGet()
            }
        }, graceMillis = 100)
        val result = scope.async(Dispatchers.Default) { try { run.execute(); null } catch (error: Exception) { error } }
        private var stopped = false
        suspend fun stop() {
            if (stopped) return
            stopped = true
            run.cancel()
            try { run.retire() } finally { result.cancelAndJoin() }
        }
    }
}
