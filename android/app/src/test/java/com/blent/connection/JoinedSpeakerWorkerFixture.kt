package com.blent.connection

import android.content.*
import android.os.*
import com.blent.*
import java.net.Socket
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment

/** Real media service, Binder protocol, worker adapter and audio wire/renderer.
 * Native device and process-death notifications are controlled synthetic evidence. */
internal class JoinedSpeakerWorkerFixture {
    val controller = Robolectric.buildService(WorkerProbeService::class.java).create()
    val service = controller.get()
    val tasks = CopyOnWriteArrayList<MediaTask.Audio>()
    val commands = CopyOnWriteArrayList<Int>()
    val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
    val socket = AtomicReference<Socket?>()
    val audible = CompletableDeferred<Unit>()
    val writeRelease = CompletableDeferred<Unit>()
    @Volatile var holdWrite = false
    val release = CompletableDeferred<Unit>()
    val cleanup = CompletableDeferred<Unit>()
    val opened = AtomicInteger()
    val closes = AtomicInteger()
    val unbound = AtomicInteger()
    @Volatile var alive = true
    @Volatile var exitRequested = false
    @Volatile var failure: String? = null
    @Volatile var failRetirement = false
    @Volatile var acceptExit = true
    private var remote: IBinder? = null
    private val player = object : SpeakerDevice {
        override val paused = false
        override suspend fun awaitReady() = awaitCancellation()
        override fun start() {}
        override fun description() = "joined synthetic speakers"
        override suspend fun write(chunk: PlaybackChunk): SpeakerWrite {
            if (chunk.samples.any { it != 0.toShort() }) {
                audible.complete(Unit)
                if (holdWrite) withContext(NonCancellable) { writeRelease.await() }
            }
            delay(2); return SpeakerWrite.Written
        }
        override fun close() { closes.incrementAndGet() }
    }
    private val endpoint = object : Binder() {
        override fun isBinderAlive() = alive
        override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { deaths.add(recipient) }
        override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int) = deaths.remove(recipient)
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            commands.add(code)
            val result = checkNotNull(remote).transact(code, data, reply, flags)
            if (code == WorkerIpc.EXIT) { exitRequested = true; finishDeath() }
            return result
        }
    }
    val context = object : ContextWrapper(RuntimeEnvironment.getApplication()) {
        override fun getApplicationContext(): Context = this
        override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
            check(intent.component?.className == SpeakerWorkerService::class.java.name)
            remote = service.onBind(intent)
            connection.onServiceConnected(intent.component, endpoint)
            return true
        }
        override fun unbindService(connection: ServiceConnection) { unbound.incrementAndGet() }
    }
    init { service.factory = { task, _, report -> native(task as MediaTask.Audio, report) } }
    private fun native(task: MediaTask.Audio, report: (WorkerEvent) -> Unit): LifecycleRun {
        tasks.add(task)
        val playback = AudioPlayback(context) { _, _ -> opened.incrementAndGet(); player }
        return object : LifecycleRun {
            override suspend fun execute() {
                failure?.let { error(it) }
                playback.run(task.endpoint, task.preferences, socket::set) { report(WorkerEvent.Status(it)) }
            }
            override fun cancel() { socket.get()?.close() }
            override suspend fun retire() {
                release.await()
                if (failRetirement) error("synthetic native release failed")
                cleanup.complete(Unit)
                finishDeath()
            }
        }
    }
    fun run(endpoint: AudioEndpoint, preferences: AudioPreferences, report: (String) -> Unit): LifecycleRun =
        AndroidWorkerRun(context, MediaTask.Audio(endpoint, preferences), report = {
            if (it is WorkerEvent.Status) report(it.text)
        }, graceMillis = 5, exitMillis = 4000)
    private fun finishDeath() { if (exitRequested && cleanup.isCompleted && acceptExit) die() }
    fun die() { alive = false; deaths.toList().forEach { it.binderDied() }; socket.get()?.close() }
    fun close() { writeRelease.complete(Unit); release.complete(Unit); die(); controller.destroy() }
}
