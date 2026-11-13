package com.blent

import android.content.Intent
import android.os.Looper
import com.blent.connection.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

/** Review evidence only. No physical device, native sockets or permanent test edits. */
@RunWith(ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
class ReviewAdbOrderingProbe {
    private enum class Timing { FOREGROUND, BACKGROUND, AFTER_RESUME }
    private fun idle() { shadowOf(Looper.getMainLooper()).idle() }
    private fun deliver() = assertTrue(TokenDelivery.store(RuntimeEnvironment.getApplication(),
        Intent().putExtra("token", "a".repeat(64))))
    private fun select(fixture: ConnectionUiFixture) {
        fixture.connect(); fixture.prompt()
        val worker = fixture.workers.single()
        worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        worker.report(ConnectionWorkerEvent.Display(displayRoute()))
    }
    private fun disconnect(fixture: ConnectionUiFixture) {
        fixture.binding.disconnect()
        fixture.workers.single().released.complete(Unit)
        idle()
    }

    @Test fun backgroundPostDisconnectDeliveryMustRemainFreshAtResume() = deliveryOrder(Timing.BACKGROUND)
    @Test fun foregroundPostDisconnectDeliveryRestoresAdb() = deliveryOrder(Timing.FOREGROUND)
    @Test fun postResumeDeliveryRestoresAdb() = deliveryOrder(Timing.AFTER_RESUME)

    private fun deliveryOrder(timing: Timing) {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            val rig = SessionDisplayRig()
            deliver(); select(fixture)
            val activity = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(activity.get(), "session", rig.session)
            try {
                activity.start().resume(); idle()
                assertEquals(displayRoute().controlPort, rig.control.routes?.controlPort)
                if (timing != Timing.FOREGROUND) activity.pause().stop()
                disconnect(fixture)
                if (timing == Timing.AFTER_RESUME) activity.start().resume()
                deliver(); idle()
                val admittedBeforeResume = rig.prefs.adbDeliveryPermitted
                if (timing == Timing.BACKGROUND) activity.start().resume()
                idle()
                println("ADB_ORDER timing=$timing delivery=${rig.prefs.hostTokenDelivery} admittedBeforeResume=$admittedBeforeResume admittedAfter=${rig.prefs.adbDeliveryPermitted} sockets=${rig.sockets.size} port=${rig.control.routes?.controlPort}")
                assertEquals("Protected delivery after USB disconnect must survive foreground transition", 8891, rig.control.routes?.controlPort)
            } finally {
                activity.pause().stop().destroy(); CameraOwner.reset(); AudioOwner.reset(); idle()
            }
        }
    }

    @Test fun disconnectAfterNullUsbSelectionMustNotLeavePreDisconnectAdbAuthority() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            val rig = SessionDisplayRig()
            deliver(); fixture.connect()
            val activity = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(activity.get(), "session", rig.session)
            try {
                activity.start().resume(); idle()
                assertNull(rig.control.routes)
                deliver(); idle()
                assertEquals(8891, rig.control.routes?.controlPort)
                disconnect(fixture)
                println("NULL_SELECTION active=${fixture.binding.state.value.active} reconnect=${fixture.binding.state.value.reconnectAllowed} delivery=${rig.prefs.hostTokenDelivery} admitted=${rig.prefs.adbDeliveryPermitted} port=${rig.control.routes?.controlPort}")
                assertNull("Disconnect must cut off previously delivered ADB authority", rig.control.routes)
            } finally {
                activity.pause().stop().destroy(); CameraOwner.reset(); AudioOwner.reset(); idle()
            }
        }
    }
}
