package com.blent

import android.media.MediaCodec
import java.util.ArrayDeque
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.shadows.ShadowMediaCodec

@Implements(MediaCodec::class)
class DrainCodecShadow : ShadowMediaCodec() {
    companion object {
        val pending = ArrayDeque<Int>()
        val releases = mutableListOf<Pair<Int, Boolean>>()
        var dequeues = 0
        var afterDequeue: () -> Unit = {}
    }
    @Implementation fun dequeueOutputBuffer(info: MediaCodec.BufferInfo, timeoutUs: Long): Int {
        assertEquals("T399: lookahead never waits for another picture", 0L, timeoutUs)
        dequeues++
        val index = pending.pollFirst() ?: -1
        info.presentationTimeUs = (index + 100).toLong()
        afterDequeue()
        return index
    }
    @Implementation override fun releaseOutputBuffer(index: Int, render: Boolean) { releases.add(index to render) }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [DrainCodecShadow::class])
class DecodedOutputDrainerTest {
    @Test fun t497_sessionReportsDiscardedDecodedOutputsAndHonorsRetirement() {
        DrainCodecShadow.pending.clear(); DrainCodecShadow.pending.addAll(listOf(1, 2))
        DrainCodecShadow.releases.clear(); DrainCodecShadow.afterDequeue = {}
        val codec = MediaCodec.createDecoderByType("video/avc")
        val receiver = VideoReceiver { error("T497 local output test") }
        val owner = CodecLifetime(codec)
        org.robolectric.util.ReflectionHelpers.setField(receiver.decoder, "mediaCodec", codec)
        org.robolectric.util.ReflectionHelpers.setField(receiver.decoder, "codecAlive", true)
        org.robolectric.util.ReflectionHelpers.setField(receiver.decoder, "lifetime", owner)
        try {
            val drainer = org.robolectric.util.ReflectionHelpers.callInstanceMethod<DecoderOutputPolicy>(
                receiver.decoder, "outputPolicy",
                org.robolectric.util.ReflectionHelpers.ClassParameter.from(MediaCodec::class.java, codec),
                org.robolectric.util.ReflectionHelpers.ClassParameter.from(DecoderCodecAccess::class.java, owner),
                org.robolectric.util.ReflectionHelpers.ClassParameter.from(DecoderOutputDriver::class.java,
                    com.blent.benchmark.replayOutputDriver("render-latest")))
            assertEquals(102, drainer.release(0, 100))
            assertEquals(2L, receiver.decoder.discardedOutputs)
            assertEquals(listOf(0 to false, 1 to false, 2 to true), DrainCodecShadow.releases)
            owner.closeAdmission()
            assertNull(drainer.release(3, 103))
            assertEquals(3, DrainCodecShadow.releases.size)
            assertEquals(2L, receiver.decoder.discardedOutputs)
        } finally { receiver.stop(); owner.awaitRetirement(2000) }
    }

    private fun exercise(latest: Boolean, pending: List<Int>, retireDuringRead: Boolean = false): Pair<Int?, List<Int>> {
        DrainCodecShadow.pending.clear(); DrainCodecShadow.pending.addAll(pending)
        DrainCodecShadow.releases.clear(); DrainCodecShadow.dequeues = 0
        var alive = true
        DrainCodecShadow.afterDequeue = { if (retireDuringRead) alive = false }
        val codec = MediaCodec.createDecoderByType("video/avc")
        val dropped = mutableListOf<Int>()
        try {
            val policy = if (latest) DecodedOutputDrainer(codec, { alive }, dropped::add)
                         else PresentDecodedOutput(codec) { alive }
            val result = policy.release(0, 100)
            return result to dropped
        } finally { codec.release() }
    }

    @Test fun t1751_currentPolicyRejectsRetiredOutputWithoutLookahead() {
        DrainCodecShadow.releases.clear(); DrainCodecShadow.dequeues = 0
        val codec = MediaCodec.createDecoderByType("video/avc")
        try {
            assertNull(PresentDecodedOutput(codec) { false }.release(0, 100))
            assertTrue(DrainCodecShadow.releases.isEmpty())
            assertEquals(0, DrainCodecShadow.dequeues)
        } finally { codec.release() }
    }

    @Test fun t1751_replayPrioritySelectionDoesNotStartThreads() {
        for (name in listOf("legacy", "socket-heap", "socket-direct", "render-latest", "callback-legacy")) {
            val thread = com.blent.benchmark.replayOutputDriver(name).thread(Runnable {})
            assertEquals(Thread.State.NEW, thread.state)
            assertEquals(Thread.MAX_PRIORITY, thread.priority)
        }
        for (name in listOf("sync-normal", "callback-supported", "callback-supported1", "callback-unhinted")) {
            val thread = com.blent.benchmark.replayOutputDriver(name).thread(Runnable {})
            assertEquals(Thread.State.NEW, thread.state)
            assertEquals(Thread.NORM_PRIORITY, thread.priority)
        }
        assertThrows(IllegalStateException::class.java) { com.blent.benchmark.replayOutputDriver("unknown") }
    }

    @Test fun t399_default_releases_every_output_without_lookahead() {
        assertEquals(100 to emptyList<Int>(), exercise(false, listOf(1, 2)))
        assertEquals(listOf(0 to true), DrainCodecShadow.releases)
        assertEquals(0, DrainCodecShadow.dequeues)
    }
    @Test fun t399_latest_preserves_decoding_and_presents_only_last_ready_output() {
        assertEquals(102 to listOf(100, 101), exercise(true, listOf(1, 2)))
        assertEquals(listOf(0 to false, 1 to false, 2 to true), DrainCodecShadow.releases)
    }
    @Test fun t399_continuous_output_cannot_starve_presentation() {
        assertEquals(103 to listOf(100, 101, 102), exercise(true, (1..8).toList()))
        assertEquals(3, DrainCodecShadow.dequeues)
        assertEquals(3 to true, DrainCodecShadow.releases.last())
    }
    @Test fun t399_sparse_or_format_change_preserves_the_available_frame() {
        for (pending in listOf(emptyList(), listOf(MediaCodec.INFO_OUTPUT_FORMAT_CHANGED))) {
            assertEquals(100 to emptyList<Int>(), exercise(true, pending))
            assertEquals(listOf(0 to true), DrainCodecShadow.releases)
        }
    }
    @Test fun t399_retirement_during_lookahead_cannot_present_an_old_surface_frame() {
        assertEquals(null to emptyList<Int>(), exercise(true, listOf(1), true))
        assertTrue(DrainCodecShadow.releases.isEmpty())
    }
}
