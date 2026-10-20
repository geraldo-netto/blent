package com.blent

import android.graphics.SurfaceTexture
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import java.io.*
import java.net.Socket
import java.net.SocketAddress
import java.net.InetSocketAddress
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.robolectric.RuntimeEnvironment
import org.robolectric.util.ReflectionHelpers

/** T1011: a 1KiB memory-only transport. JDK pipes and an unconnected Socket stay
 * owned until close and the exact original receiver job has joined. */
internal class JoinedDisplaySocket(private val port: Int) : Socket() {
    private val incoming = PipedInputStream(1024)
    private val producer = PipedOutputStream(incoming)
    val authentication = ByteArrayOutputStream()
    override fun connect(endpoint: SocketAddress?, timeout: Int) {
        val target = checkNotNull(endpoint as? InetSocketAddress)
        require(target.address.isLoopbackAddress); require(target.port == port); require(timeout > 0)
    }
    override fun setTcpNoDelay(value: Boolean) { check(value) }
    override fun setSoTimeout(value: Int) { require(value > 0) }
    override fun setReceiveBufferSize(value: Int) { require(value > 0) }
    override fun getInputStream(): InputStream = incoming
    override fun getOutputStream(): OutputStream = authentication
    fun supply() {
        // Independent H.264 NAL markers: SPS/PPS config followed by an IDR unit.
        val config = byteArrayOf(0, 0, 0, 1, 0x67, 1, 0, 0, 0, 1, 0x68, 2)
        packet(0, config)
        packet(1, byteArrayOf(0, 0, 0, 1, 0x65, 3), 7)
    }
    private fun packet(type: Int, payload: ByteArray, sequence: Int? = null) {
        require(payload.size < 128)
        val output = DataOutputStream(producer)
        output.writeInt(1+payload.size+if (sequence == null) 0 else 4)
        output.writeByte(type); sequence?.let(output::writeInt); output.write(payload); output.flush()
    }
    @Synchronized override fun close() {
        cleanupAll(incoming::close, producer::close, { super.close() }).firstOrNull()?.let { throw it }
    }
}

internal class JoinedDisplayWorkerFixture {
    /** Hosted API/job retirement and a separately held simulated process fence.
     * Actual device/process proof remains T914. */
    class Original(val task: MediaTask.Display, surface: Surface, report: (WorkerEvent) -> Unit) : LifecycleRun {
        val socket = JoinedDisplaySocket(task.routes.videoPort)
        val fence = CompletableDeferred<Unit>()
        val nativeRetired = CompletableDeferred<Unit>()
        @Volatile var settled = false
        @Volatile var originalJob: Job? = null; private set
        private val receiver = VideoReceiver(task.routes) { socket }.apply {
            setStreamFormat(task.format); setVideoLiveness(task.liveness); setNativeSurface(surface)
        }
        private val actual = DisplayWorkerRun(receiver, report)
        override suspend fun execute() = coroutineScope {
            val work = launch(start = CoroutineStart.UNDISPATCHED) { actual.execute() }
            captureJob(); work.join()
        }
        private fun captureJob() = synchronized(receiver) {
            val current = ReflectionHelpers.getField<Job?>(receiver, "job")
            if (current != null) {
                check(originalJob == null || originalJob === current)
                originalJob = current
            }
        }
        override fun cancel() { captureJob(); actual.cancel(); socket.close() }
        override suspend fun retire() {
            try { actual.retire() } finally { socket.close(); originalJob?.join() }
            nativeRetired.complete(Unit)
            fence.await(); settled = true
        }
    }
    val scope = CoroutineScope(SupervisorJob()+Dispatchers.Unconfined)
    val originals = CopyOnWriteArrayList<Original>()
    private val texture = SurfaceTexture(0)
    private val surface = Surface(texture)
    val receiver = RemoteDisplay(RuntimeEnvironment.getApplication(), scope) { task, output, report ->
        check(originals.size < 2)
        Original(task, output, report).also { originals.add(it) }
    }
    val view = object : SurfaceView(RuntimeEnvironment.getApplication()) {
        override fun getHolder(): SurfaceHolder = object : SurfaceHolder by super.getHolder() {
            override fun getSurface(): Surface = this@JoinedDisplayWorkerFixture.surface
        }
    }
    suspend fun close() {
        originals.forEach { it.fence.complete(Unit) }
        receiver.stop(); receiver.onSurfaceDestroyed()
        checkNotNull(scope.coroutineContext[Job]).cancelAndJoin()
        awaitDecoderRetirement()
        assertTrue(originals.all { it.settled })
        assertTrue(originals.all { it.socket.isClosed })
        assertTrue(originals.all { it.originalJob?.isCompleted != false })
        assertTrue(JoinedDisplayCodecShadow.instances.all { it.released })
        surface.release(); texture.release()
        // T1172: this sequential SDK fixture owns the bounded codec history.
        // Return its metadata capacity only after actual original settlement.
        JoinedDisplayCodecShadow.clear()
    }
}
