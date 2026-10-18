package com.blent

import android.media.*
import android.os.Handler
import android.view.Surface
import java.nio.ByteBuffer
import java.util.concurrent.CopyOnWriteArrayList
import org.junit.Assert.*
import org.robolectric.annotation.*
import org.robolectric.shadows.ShadowMediaCodec

/** T1011: two codec API owners with bounded borrowed input and immutable history.
 * No real decode/presentation is simulated by this native boundary fixture. */
@Implements(MediaCodec::class)
class JoinedDisplayCodecShadow : ShadowMediaCodec() {
    internal data class Format(val mime: String, val width: Int, val height: Int)
    internal data class Input(val bytes: ByteArray, val flags: Int, val sequence: Long)
    companion object {
        internal val instances = CopyOnWriteArrayList<JoinedDisplayCodecShadow>()
        @JvmStatic @Resetter fun clear() { instances.clear() }
    }
    @RealObject lateinit var codec: MediaCodec
    private val input = ByteBuffer.allocate(1024)
    internal val queued = CopyOnWriteArrayList<Input>()
    internal lateinit var format: Format
    @Volatile internal lateinit var listener: MediaCodec.OnFrameRenderedListener
    @Volatile internal var released = false
    @Volatile private var live = false
    @Implementation fun configure(format: MediaFormat, surface: Surface?, crypto: MediaCrypto?, flags: Int) {
        check(surface?.isValid == true); check(crypto == null); check(flags == 0)
        check(instances.size < 2)
        this.format = Format(checkNotNull(format.getString(MediaFormat.KEY_MIME)),
            format.getInteger(MediaFormat.KEY_WIDTH), format.getInteger(MediaFormat.KEY_HEIGHT))
        instances.add(this)
    }
    @Implementation fun setVideoScalingMode(mode: Int) { check(mode == MediaCodec.VIDEO_SCALING_MODE_SCALE_TO_FIT) }
    @Implementation fun setOnFrameRenderedListener(listener: MediaCodec.OnFrameRenderedListener, handler: Handler) {
        check(handler.looper.thread.isAlive); this.listener = listener
    }
    @Implementation fun start() { check(!released); live = true }
    @Implementation fun stop() { live = false }
    @Implementation fun release() { check(!released); live = false; released = true }
    @Implementation fun dequeueInputBuffer(timeoutUs: Long): Int { require(timeoutUs >= 0); check(live); return 0 }
    @Implementation fun getInputBuffer(index: Int): ByteBuffer { require(index == 0); check(live); return input }
    @Implementation fun queueInputBuffer(index: Int, offset: Int, size: Int, presentationTimeUs: Long, flags: Int) {
        check(live); require(index == 0); require(offset == 0); require(size in 1..1024)
        check(queued.size < 4)
        val bytes = ByteArray(size)
        input.duplicate().apply { position(offset) }.get(bytes)
        queued.add(Input(bytes, flags, presentationTimeUs))
    }
    // Native output arguments retain their ABI; this fixture produces no output.
    @Suppress("UNUSED_PARAMETER")
    @Implementation fun dequeueOutputBuffer(info: MediaCodec.BufferInfo, timeoutUs: Long): Int {
        require(timeoutUs >= 0); Thread.sleep(1); return -1
    }
}
