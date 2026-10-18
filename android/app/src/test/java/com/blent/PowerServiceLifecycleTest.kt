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
class PowerServiceLifecycleTest {
    @Test fun t1060OldEnvelopeCannotPromoteOrRevokeNewerQueuedPolicy() = runBlocking {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower())
        val intent = rig.app.starts.single()
        val controller = ServiceController.of(StreamingService(rig.runtime), Intent()).create()
        val service = controller.get()
        try {
            service.onStartCommand(intent, 0, 1); rig.drain()
            assertEquals(1, rig.native.size)
            rig.runtime.publish(owner, StreamingPower(true, true))
            shadowOf(service).setThrowInStartForeground(SecurityException("T1060 promotion must not run for stale envelope"))
            service.onStartCommand(intent, 0, 2)
            assertTrue("T1060 stale envelope revoked newer queued power", rig.runtime.owns(owner))
            assertEquals(1, rig.native.size)
            shadowOf(service).setThrowInStartForeground(null)
            shadowOf(android.os.Looper.getMainLooper()).idle(); rig.drain()
            assertEquals(2, rig.native.size)
        } finally { controller.destroy(); rig.finish(owner) }
    }
    @Test fun t1060OwnerlessStaleEnvelopeCannotLeaveFreshComponentForeground() = runBlocking {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower())
        val intent = rig.app.starts.single()
        val original = ServiceController.of(StreamingService(rig.runtime), Intent()).create()
        original.get().onStartCommand(intent, 0, 1); rig.drain()
        rig.runtime.close(owner); original.destroy(); rig.drain()
        val fresh = ServiceController.of(StreamingService(rig.runtime), Intent()).create()
        try {
            fresh.get().onStartCommand(intent, 0, 2)
            assertTrue("T1060 ownerless stale envelope retained foreground component", shadowOf(fresh.get()).isStoppedBySelf)
            assertEquals(1, rig.native.size)
        } finally { fresh.destroy(); rig.finish(owner) }
    }
}
