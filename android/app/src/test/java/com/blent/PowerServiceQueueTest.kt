package com.blent

import android.content.Intent
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.android.controller.ServiceController
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
@LooperMode(LooperMode.Mode.PAUSED)
class PowerServiceQueueTest {
    private class Rig(val queue: PowerSchedule = PowerRuntimeFixture.Schedule()) {
        val power = PowerRuntimeFixture()
        val owner = power.runtime.open(StreamingPower())
        val controller = ServiceController.of(StreamingService(power.runtime, queue), Intent()).create()
        init { controller.get().onStartCommand(power.app.starts.single(), 0, 1); power.drain() }
        suspend fun close() { controller.destroy(); power.finish(owner) }
    }
    @Test fun t1060DestroyFencesOriginalQueuedCallbackAndRequiresFreshSuccessorNotice() = runBlocking {
        val clock = PowerRuntimeFixture.Schedule()
        val rig = Rig(clock)
        var fresh: org.robolectric.android.controller.ServiceController<StreamingService>? = null
        try {
            rig.power.runtime.publish(rig.owner, StreamingPower(true, true))
            val original = clock.entries.single().action
            val next = rig.power.runtime.open(StreamingPower(true, true, StreamTransport.USB))
            rig.controller.destroy()
            assertTrue(clock.entries.isEmpty())
            assertEquals(2, rig.power.app.starts.size)
            fresh = ServiceController.of(StreamingService(rig.power.runtime), Intent()).create()
            fresh.get().onStartCommand(rig.power.app.starts.last(), 0, 2); rig.power.drain()
            original.run()
            assertTrue(rig.power.runtime.owns(next))
            assertEquals(1, rig.power.native.size)
            rig.power.runtime.close(next)
        } finally { fresh?.destroy(); rig.close() }
    }
    @Test fun t1060UnknownCancelRetainsOriginalAdmissionUntilItsActualLateCallback() = runBlocking {
        val clock = PowerRuntimeFixture.Schedule()
        val rig = Rig(clock)
        try {
            rig.power.runtime.publish(rig.owner, StreamingPower(true, true))
            val original = clock.entries.single().action
            clock.cancelFailure = true
            rig.controller.destroy()
            val next = rig.power.runtime.open(StreamingPower())
            assertEquals("T1060 unknown callback cancellation lost original admission", 1, rig.power.app.starts.size)
            assertEquals(1, rig.power.native.size)
            original.run()
            assertEquals(2, rig.power.app.starts.size)
            original.run()
            assertEquals(2, rig.power.app.starts.size)
            rig.power.runtime.close(next)
            rig.power.runtime.canceled(checkNotNull(PowerServiceNotice.read(rig.power.app.starts.last())).notice)
        } finally { clock.cancelFailure = false; rig.close() }
    }
    @Test fun t1060AcceptedThenThrowKeepsOriginalCallbackAndCannotAdmitAnother() = runBlocking {
        val clock = PowerRuntimeFixture.Schedule()
        val rig = Rig(clock)
        try {
            clock.postFailure = true
            rig.power.runtime.publish(rig.owner, StreamingPower(true, true))
            assertFalse(rig.power.runtime.owns(rig.owner))
            val next = rig.power.runtime.open(StreamingPower())
            assertEquals(1, clock.entries.size)
            val original = clock.entries.single().action
            clock.postFailure = false
            clock.entries.clear()
            original.run()
            assertEquals(1, clock.entries.size) // Fresh successor owns a distinct queued callback.
            assertTrue(rig.power.runtime.owns(next))
            assertTrue(rig.power.native.size <= 1)
            rig.power.runtime.close(next)
        } finally { rig.close() }
    }
    @Test fun t1060KnownPostRejectionFencesRefusedCallbackAndClosesOriginalOwner() = runBlocking {
        val clock = PowerRuntimeFixture.Schedule()
        var captured: Runnable? = null
        val queue = object : PowerSchedule by clock {
            override fun post(delayMillis: Long, action: Runnable): Boolean { captured = action; return false }
        }
        val rig = Rig(queue)
        try {
            rig.power.runtime.publish(rig.owner, StreamingPower(true, true))
            assertFalse(rig.power.runtime.owns(rig.owner))
            assertTrue(shadowOf(rig.controller.get()).isStoppedBySelf)
            val count = rig.power.native.size
            checkNotNull(captured).run()
            assertEquals(count, rig.power.native.size)
        } finally { rig.close() }
    }
    @Test fun t1060FrameworkDefaultComponentPromotesBeforeCommandAndStopsWithoutAnOwner() {
        val controller = org.robolectric.Robolectric.buildService(StreamingService::class.java).create()
        try {
            assertNotNull(shadowOf(controller.get()).lastForegroundNotification)
            assertEquals(android.app.Service.START_NOT_STICKY, controller.get().onStartCommand(null, 0, 1))
            assertTrue(shadowOf(controller.get()).isStoppedBySelf)
        } finally { controller.destroy() }
    }
    @Test fun t1060OwnerlessNullAndForeignNativeEnvelopesCannotAcquire() = runBlocking {
        val rig = PowerRuntimeFixture()
        val controller = ServiceController.of(StreamingService(rig.runtime), Intent()).create()
        try {
            assertNull(controller.get().onBind(null))
            for (intent in listOf(null, Intent(), StreamingPower().intent(rig.app))) {
                assertEquals(android.app.Service.START_NOT_STICKY, controller.get().onStartCommand(intent, 0, 1))
                assertTrue(shadowOf(controller.get()).isStoppedBySelf)
            }
            assertTrue(rig.native.isEmpty())
        } finally { controller.destroy(); rig.scope.cancel() }
    }
}
