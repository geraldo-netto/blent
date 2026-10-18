package com.blent

import android.media.MediaCodec
import com.blent.DisplayWakeIntegrationTest.Rig
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [JoinedDisplayCodecShadow::class])
class JoinedDisplayActivityTest {
    @get:org.junit.Rule val inventory = DecoderInventoryRule()
    private fun codec(index: Int) = JoinedDisplayCodecShadow.instances[index]
    private fun currentInputs(index: Int) {
        val input = codec(index).queued
        assertEquals(2, input.size)
        assertEquals(MediaCodec.BUFFER_FLAG_CODEC_CONFIG, input[0].flags)
        assertEquals(0x67, input[0].bytes[4].toInt())
        assertEquals(0, input[1].flags); assertEquals(0x65, input[1].bytes[4].toInt())
        assertEquals(7L, input[1].sequence)
    }
    @Test fun t1011CurrentRouteResumesOnlyAfterOriginalNativeFenceWithFreshCodecHeadersAndKeyframe() = runBlocking {
        val native = JoinedDisplayWorkerFixture()
        val rig = Rig(native.receiver)
        val power = PowerRuntimeFixture()
        val binding = StreamingPowerBinding(power.app, power.runtime)
        val writes = ArrayList<Boolean>()
        val window = WindowPowerBinding(writes::add)
        val receipts = ArrayList<RenderAcknowledgement>()
        val originalRender = native.receiver.onFrameRendered
        native.receiver.onFrameRendered = { receipts.add(it); originalRender?.invoke(it) }
        rig.session.selectDisplay(DisplayEndpoint("cd".repeat(32), videoPort = 23456, controlPort = 23457))
        rig.session.surfaceReady(native.view)
        binding.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        window.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        val owner = checkNotNull(PowerServiceNotice.read(power.app.starts.single())).notice.original.owner
        try {
            rig.start(); rig.signal(false); rig.until { rig.session.powerNow().display == DisplayWakeState.PAUSED }
            power.attachFirst(); assertTrue(native.originals.isEmpty()); assertTrue(power.native.isEmpty())
            rig.signal(true, 2, 0, false)
            rig.until(power::drain) { native.originals.size == 1 && codecReady(0) && writes.last() }
            val first = native.originals.single()
            rig.until { first.socket.authentication.size() == 64 }
            assertEquals("cd".repeat(32), first.socket.authentication.toString("US-ASCII"))
            first.socket.supply(); rig.until { codec(0).queued.size == 2 }; currentInputs(0)
            codec(0).listener.onFrameRendered(codec(0).codec, 7, 0)
            assertEquals(1, receipts.size); assertTrue(receipts.single().current())
            rig.signal(false, 3, 1, false)
            rig.until(power::drain) { first.nativeRetired.isCompleted && !writes.last() }
            assertFalse(receipts.single().current()); assertTrue(codec(0).released)
            repeat(16) { rig.signal(true, 4L+it, 2L+it, false) }
            rig.drain(); assertEquals(1, native.originals.size)
            val prior = rig.sockets.last()
            rig.session.selectDisplay(DisplayEndpoint("ef".repeat(32), videoPort = 34567, controlPort = 34568))
            rig.opened(); rig.signal(true)
            rig.until { writes.last() }
            prior.second.onMessage(prior.first, """{"status":"connected","pen_only":true}""")
            codec(0).listener.onFrameRendered(codec(0).codec, 8, 0)
            assertEquals(1, receipts.size); assertEquals(1, native.originals.size)
            assertFalse(first.settled)
            first.fence.complete(Unit)
            rig.until(power::drain) { native.originals.size == 2 && codecReady(1) }
            val next = native.originals.last()
            assertTrue(first.settled); assertTrue(checkNotNull(first.originalJob).isCompleted)
            assertTrue(first.socket.isClosed)
            rig.until { next.socket.authentication.size() == 64 }
            assertEquals("ef".repeat(32), next.socket.authentication.toString("US-ASCII"))
            assertEquals(34567, next.task.routes.videoPort)
            assertEquals(34568, next.task.routes.controlPort)
            assertEquals(first.task.format, next.task.format)
            assertTrue(codec(1).queued.isEmpty())
            assertEquals(64, codec(1).format.width)
            next.socket.supply(); rig.until { codec(1).queued.size == 2 }; currentInputs(1)
            codec(1).listener.onFrameRendered(codec(1).codec, 7, 0)
            assertEquals(2, receipts.size); assertTrue(receipts.last().current())
            rig.session.stop()
            next.fence.complete(Unit)
            rig.until(power::drain) { !writes.last() && next.settled }
            assertFalse(receipts.last().current()); assertEquals(2, native.originals.size)
        } finally {
            window.stop(); binding.stop()
            try { rig.close() }
            finally { try { native.close() } finally { power.finish(owner); checkNotNull(power.scope.coroutineContext[Job]).cancelAndJoin(); rig.drain() } }
        }
    }
    private fun codecReady(index: Int) = JoinedDisplayCodecShadow.instances.size > index && codec(index).listenerReady()
    private fun JoinedDisplayCodecShadow.listenerReady() = runCatching { listener }.isSuccess
}
