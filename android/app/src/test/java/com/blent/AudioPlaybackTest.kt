package com.blent

import java.net.ServerSocket
import java.net.Socket
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AudioSpeakerTest.RoutedTrack::class])
class AudioPlaybackTest {
    private fun endpoint(port: Int) = AudioEndpoint("a".repeat(64), port, 2, 1, 40, false)
    private fun grant(endpoint: AudioEndpoint): ByteArray = endpoint.token.toByteArray() +
        ByteBuffer.allocate(92).put("BLAUD001".toByteArray()).put("b".repeat(64).toByteArray())
            .putLong(1).put(2).put(2).putInt(48000).putShort(480).put(1).put(0).putShort(40).array()
    private fun packet(sequence: Long): ByteArray {
        val bytes = ByteBuffer.allocate(1948).putLong(1).putLong(sequence).putLong(sequence * 10)
            .putShort(1920).put(2).put(0).order(java.nio.ByteOrder.LITTLE_ENDIAN)
        repeat(480) { bytes.putShort(1234).putShort(-4321) }
        return bytes.array()
    }
    private class Player : SpeakerDevice {
        var starts = 0; var closes = 0; val audible = AtomicBoolean(false)
        override val paused = false
        override suspend fun awaitReady() {}
        override fun start() { starts++ }
        override fun description() = "fixture speakers"
        override suspend fun write(chunk: PlaybackChunk): SpeakerWrite {
            if (chunk.samples.any { it != 0.toShort() }) {
                assertEquals(1234.toShort(), chunk.samples[0]); assertEquals((-4321).toShort(), chunk.samples[1])
                audible.set(true); error("fixture complete")
            }
            delay(5); return SpeakerWrite.Written
        }
        override fun close() { closes++ }
    }
    @Test fun t809_localAudioWorkerPublishesStatusAndReturnsAfterStop() = runBlocking(Dispatchers.IO) {
        ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort)
            val peer = async(Dispatchers.IO) { server.accept().use { socket ->
                socket.soTimeout = 2000; socket.getInputStream().readNBytes(76)
                socket.getOutputStream().write(grant(endpoint))
                assertEquals(-1, socket.getInputStream().read())
            } }
            val events = mutableListOf<WorkerEvent>()
            lateinit var run: LifecycleRun
            run = LocalMediaRuns.create(RuntimeEnvironment.getApplication(),
                MediaTask.Audio(endpoint, AudioPreferences(builtIn = false)), null) {
                events.add(it); run.cancel()
            }
            withTimeout(3000) { run.execute() }; run.retire(); peer.await()
            assertTrue(events.single() is WorkerEvent.Status)
        }
    }
    @Test fun t801_stopDuringNativeStartCompletesWithoutLaunchingAudioWork() = runBlocking(Dispatchers.Unconfined) {
        ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort)
            var attached: Socket? = null
            var closed = 0
            val peer = async(Dispatchers.IO) { server.accept().use { socket ->
                socket.getInputStream().readNBytes(76); socket.getOutputStream().write(grant(endpoint))
                assertEquals(-1, socket.getInputStream().read())
            } }
            val player = object : SpeakerDevice {
                override val paused = false
                override fun start() { attached!!.close() }
                override suspend fun awaitReady() { error("T801 closed stream awaited focus") }
                override fun description(): String = error("T801 closed stream sampled device")
                override suspend fun write(chunk: PlaybackChunk): SpeakerWrite = error("T801 closed stream wrote PCM")
                override fun close() { closed++ }
            }
            val inline = object : CoroutineDispatcher() {
                override fun dispatch(context: kotlin.coroutines.CoroutineContext, block: Runnable) = block.run()
            }
            AudioPlayback(RuntimeEnvironment.getApplication(), inline) { _, _ -> player }
                .run(endpoint, AudioPreferences(), { attached = it }, { error("T801 closed stream published status") })
            peer.await()
            assertNull(attached); assertEquals(1, closed)
        }
    }

    @Test fun t801_pausedPlaybackDoesNotPollNativeClockAndLocalStopStillCompletes() = runBlocking(Dispatchers.IO) {
        withTimeout(3000) { ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort)
            val attached = CompletableDeferred<Socket>()
            val paused = CompletableDeferred<Unit>()
            val reads = java.util.concurrent.atomic.AtomicInteger()
            var closes = 0
            val player = object : SpeakerDevice {
                override val paused = true
                override suspend fun awaitReady() { awaitCancellation() }
                override fun start() {}
                override fun description() = "paused fixture"
                override fun clockSample(): AudioClockSample? { reads.incrementAndGet(); return null }
                override suspend fun write(chunk: PlaybackChunk) = SpeakerWrite.Paused
                override fun close() { closes++ }
            }
            val peer = async { server.accept().use { socket ->
                socket.getInputStream().readNBytes(76); socket.getOutputStream().write(grant(endpoint))
                assertEquals(-1, socket.getInputStream().read())
            } }
            val playback = AudioPlayback(RuntimeEnvironment.getApplication()) { _, _ -> player }
            val run = async { playback.run(endpoint, AudioPreferences(), { if (it != null) attached.complete(it) }, { if (it.contains("paused")) paused.complete(Unit) }) }
            paused.await(); delay(50)
            attached.await().close()
            run.await(); peer.await()
            assertEquals("T801 native timestamp polled while paused", 0, reads.get())
            assertEquals(1, closes)
        } }
    }

    @Test fun t719_localStopReturnsNormallyAndReleasesNativePlayback() = runBlocking(Dispatchers.IO) {
        ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort); var attached: Socket? = null; var writes = 0
            val listener = org.robolectric.shadows.ShadowAudioTrack.OnAudioDataWrittenListener { _, _, _ ->
                writes++; attached!!.close()
            }
            org.robolectric.shadows.ShadowAudioTrack.addAudioDataListener(listener)
            val peer = async(Dispatchers.IO) { server.accept().use { socket ->
                socket.soTimeout = 2000; socket.getInputStream().readNBytes(76)
                socket.getOutputStream().write(grant(endpoint))
                assertEquals(-1, socket.getInputStream().read())
            } }
            try {
                AudioPlayback(RuntimeEnvironment.getApplication()).run(endpoint, AudioPreferences(builtIn = false), { attached = it }, {})
                peer.await(); assertEquals(1, writes); assertNull(attached)
                val manager = RuntimeEnvironment.getApplication().getSystemService(android.media.AudioManager::class.java)
                assertNotNull(Shadows.shadowOf(manager).lastAbandonedAudioFocusRequest)
            } finally { org.robolectric.shadows.ShadowAudioTrack.removeAudioDataListener(listener) }
        }
    }
    @Test fun t719_authenticatedStereoPlaybackClosesEveryOwnedResource() = runBlocking(Dispatchers.IO) {
        withTimeout(5000) { ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort); val player = Player()
            var attached: Socket? = null; var status = ""
            val peer = async(Dispatchers.IO) { server.accept().use { socket ->
                socket.soTimeout = 2000
                val request = socket.getInputStream().readNBytes(76)
                assertArrayEquals(byteArrayOf(7, 0, 2, 1), request.copyOfRange(72, 76))
                assertEquals(0, player.starts)
                socket.getOutputStream().write(grant(endpoint))
                repeat(4) { socket.getOutputStream().write(packet(it.toLong())) }
                assertEquals(-1, socket.getInputStream().read())
            } }
            val playback = AudioPlayback(RuntimeEnvironment.getApplication()) { _, _ -> player }
            try { playback.run(endpoint, AudioPreferences(), { attached = it }, { status = it }); fail("fixture did not end") }
            catch (error: IllegalStateException) { assertEquals("fixture complete", error.message) }
            peer.await(); assertTrue(player.audible.get()); assertEquals(1, player.starts)
            assertEquals(1, player.closes); assertNull(attached); assertTrue(status.contains("Speaker sharing"))
        } }
    }
    @Test fun t719_nativeOpenFailureAdvertisesUnsupported() = runBlocking(Dispatchers.IO) {
        ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort)
            val peer = async(Dispatchers.IO) { server.accept().use { it.getInputStream().readNBytes(76) } }
            var attached: Socket? = null
            val playback = AudioPlayback(RuntimeEnvironment.getApplication()) { _, _ -> error("unavailable") }
            try { playback.run(endpoint, AudioPreferences(), { attached = it }, {}); fail("native failure hidden") }
            catch (error: IllegalStateException) { assertEquals("unavailable", error.message) }
            assertEquals(0, peer.await()[72].toInt()); assertNull(attached)
        }
    }
    @Test fun t719_fragmentedFrameCannotExtendCompleteReadDeadline() = runBlocking(Dispatchers.IO) {
        ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort); val player = Player()
            val peer = async(Dispatchers.IO) { server.accept().use { socket ->
                socket.getInputStream().readNBytes(76); socket.getOutputStream().write(grant(endpoint))
                try { for (byte in packet(0).take(30)) { socket.getOutputStream().write(byte.toInt()); delay(40) } }
                catch (_: java.io.IOException) {}
            } }
            val playback = AudioPlayback(RuntimeEnvironment.getApplication()) { _, _ -> player }
            val began = System.nanoTime()
            try { playback.run(endpoint, AudioPreferences(), {}, {}); fail("partial bytes kept playback alive") }
            catch (_: java.net.SocketTimeoutException) {}
            assertTrue((System.nanoTime() - began) / 1_000_000 < 1500)
            assertFalse(player.audible.get()); assertEquals(1, player.closes); peer.await()
        }
    }
    /** Records completed dispatcher turns, avoiding a scheduling/sleep assertion. */
    private class ReadDispatcher : CoroutineDispatcher() {
        val current = ThreadLocal<Job?>()
        private val turns = java.util.IdentityHashMap<Job, Int>()
        @Synchronized fun completed(job: Job) = turns[job] ?: 0
        @Synchronized private fun finished(job: Job) { turns[job] = completed(job) + 1 }
        override fun dispatch(context: kotlin.coroutines.CoroutineContext, block: Runnable) {
            Dispatchers.IO.dispatch(context) {
                val job = checkNotNull(context[Job])
                current.set(job)
                try { block.run() } finally { current.remove(); finished(job) }
            }
        }
    }
    /** Native close is observable before a blocked read delivers its failure. */
    private class DelayedReadFailure(grant: ByteArray, private val dispatcher: ReadDispatcher) : Socket() {
        val entered = CompletableDeferred<Unit>()
        val release = java.util.concurrent.CountDownLatch(1)
        val initial = java.util.concurrent.atomic.AtomicReference<Job>()
        val reading = java.util.concurrent.atomic.AtomicReference<Job>()
        private val closed = AtomicBoolean()
        val closes = java.util.concurrent.atomic.AtomicInteger()
        private val input = object : java.io.InputStream() {
            private val prefix = java.io.ByteArrayInputStream(grant)
            override fun read(): Int {
                val one = ByteArray(1)
                return if (read(one, 0, 1) == -1) -1 else one[0].toInt() and 255
            }
            override fun read(bytes: ByteArray, offset: Int, length: Int): Int {
                val count = prefix.read(bytes, offset, length)
                if (count >= 0) { initial.compareAndSet(null, dispatcher.current.get()); return count }
                reading.compareAndSet(null, checkNotNull(dispatcher.current.get()))
                entered.complete(Unit)
                check(release.await(3, java.util.concurrent.TimeUnit.SECONDS)) { "T964 read fixture not released" }
                throw java.net.SocketTimeoutException("controlled native read timeout")
            }
        }
        override fun connect(endpoint: java.net.SocketAddress, timeout: Int) {}
        override fun setTcpNoDelay(on: Boolean) {}
        override fun getInputStream() = input
        override fun getOutputStream() = java.io.ByteArrayOutputStream()
        override fun isClosed() = closed.get()
        override fun close() { if (closed.compareAndSet(false, true)) closes.incrementAndGet() }
    }
    private suspend fun untilTurn(condition: () -> Boolean) = withTimeout(3000) { while (!condition()) delay(1) }

    @Test fun t964RendererCompletionCannotCancelOrHidePendingNativeReadFailure() = runBlocking {
        val dispatcher = ReadDispatcher()
        val endpoint = endpoint(12345)
        val socket = DelayedReadFailure(grant(endpoint), dispatcher)
        val rendering = CompletableDeferred<Job>()
        var playerCloses = 0
        val player = object : SpeakerDevice {
            override val paused = false
            override suspend fun awaitReady() {}
            override fun start() {}
            override fun description() = "T964 controlled reader"
            override suspend fun write(chunk: PlaybackChunk): SpeakerWrite {
                rendering.complete(checkNotNull(currentCoroutineContext()[Job]))
                delay(1)
                return SpeakerWrite.Written
            }
            override fun close() { playerCloses++ }
        }
        val playback = AudioPlayback(RuntimeEnvironment.getApplication(), dispatcher, { socket }) { _, _ -> player }
        val outcome = async { runCatching { playback.run(endpoint, AudioPreferences(), {}, {}) } }
        try {
            withTimeout(3000) { socket.entered.await(); rendering.await() }
            val coordinator = checkNotNull(socket.initial.get()).children.single()
            val before = dispatcher.completed(coordinator)
            socket.close() // Models the timeout watchdog's close, before read delivery.
            withTimeout(3000) { rendering.await().join() }
            untilTurn { dispatcher.completed(coordinator) > before }
            assertFalse("T964 renderer completion cancelled the original pending reader", socket.reading.get().isCancelled)
            assertFalse("T964 pending native read already reported completed", outcome.isCompleted)
            socket.release.countDown()
            val failure = withTimeout(3000) { outcome.await().exceptionOrNull() }
            assertTrue("T964 native timeout was hidden: $failure", failure is java.net.SocketTimeoutException)
            assertEquals(1, playerCloses)
            assertEquals(1, socket.closes.get())
        } finally {
            socket.release.countDown(); socket.close()
            withTimeout(3000) { outcome.join() }
        }
    }
    @Test fun t719_stopWhileGrantPendingNeverStartsPlayback() = runBlocking(Dispatchers.IO) {
        ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort); val player = Player(); val attached = CompletableDeferred<Socket>()
            val ready = CompletableDeferred<Unit>()
            val peer = async(Dispatchers.IO) { server.accept().use { socket ->
                socket.getInputStream().readNBytes(76); ready.complete(Unit)
                assertEquals(-1, socket.getInputStream().read())
            } }
            val playback = AudioPlayback(RuntimeEnvironment.getApplication()) { _, _ -> player }
            val run = async { try { playback.run(endpoint, AudioPreferences(), { if (it != null) attached.complete(it) }, {}) }
                catch (_: java.net.SocketException) {} }
            ready.await(); attached.await().close(); run.await(); peer.await()
            assertEquals(0, player.starts); assertEquals(1, player.closes)
        }
    }
    @Test fun t719_transientFocusReportsPauseAndResumesOnlyCurrentSession() = runBlocking(Dispatchers.IO) {
        ServerSocket(0).use { server ->
            val endpoint = endpoint(server.localPort); val resumed = CompletableDeferred<Unit>()
            var sawPause = false; var closes = 0
            val player = object : SpeakerDevice {
                override var paused = true
                override suspend fun awaitReady() { delay(10); paused = false }
                override fun start() {}
                override fun description() = "resumed fixture"
                override suspend fun write(chunk: PlaybackChunk): SpeakerWrite {
                    if (paused) { delay(10); paused = false; return SpeakerWrite.Paused }
                    if (chunk.samples[0] != 0.toShort()) error("resumed")
                    delay(5); return SpeakerWrite.Written
                }
                override fun close() { closes++ }
            }
            val peer = async(Dispatchers.IO) { server.accept().use { socket ->
                socket.soTimeout = 2000; socket.getInputStream().readNBytes(76)
                socket.getOutputStream().write(grant(endpoint)); resumed.await()
                repeat(4) { socket.getOutputStream().write(packet(it.toLong())) }
                assertEquals(-1, socket.getInputStream().read())
            } }
            val playback = AudioPlayback(RuntimeEnvironment.getApplication()) { _, _ -> player }
            try { playback.run(endpoint, AudioPreferences(), {}, {
                if (it.contains("paused")) sawPause = true else resumed.complete(Unit)
            }); fail("resumed fixture did not run") }
            catch (error: IllegalStateException) { assertEquals("resumed", error.message) }
            peer.await(); assertTrue(sawPause); assertEquals(1, closes)
        }
    }
}
