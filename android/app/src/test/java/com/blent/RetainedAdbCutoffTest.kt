package com.blent

import android.content.Intent
import android.os.Bundle
import android.os.Looper
import com.blent.connection.ConnectionDisplayBinding
import com.blent.connection.ConnectionUiFixture
import com.blent.connection.ConnectionView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
class RetainedAdbCutoffTest {
    private enum class Cutoff { DISCONNECT, DENY, REVOKE }

    @Test fun t1414RecreatedActivityKeepsDisconnect() = recreate(Cutoff.DISCONNECT)
    @Test fun t1414RecreatedActivityKeepsDenial() = recreate(Cutoff.DENY)
    @Test fun t1414RecreatedActivityKeepsRevocation() = recreate(Cutoff.REVOKE)
    @Test fun t1414PostCutoffDeliveryAllowsRecreatedActivity() = recreate(Cutoff.DISCONNECT, true)

    private fun deliver() = assertTrue(TokenDelivery.store(RuntimeEnvironment.getApplication(),
        Intent().putExtra("token", "a".repeat(64))))

    private fun cutOff(fixture: ConnectionUiFixture, cutoff: Cutoff) {
        fixture.prompt()
        val choice = checkNotNull(fixture.binding.state.value.peer)
        when (cutoff) {
            Cutoff.DISCONNECT -> fixture.binding.disconnect()
            Cutoff.DENY -> assertTrue(fixture.binding.decide(choice, false, false))
            Cutoff.REVOKE -> assertTrue(fixture.binding.revoke(choice))
        }
        fixture.workers.single().released.complete(Unit)
        assertFalse(fixture.binding.state.value.active)
        assertFalse(fixture.binding.state.value.reconnectAllowed)
        shadowOf(Looper.getMainLooper()).idle()
    }

    private fun recreate(cutoff: Cutoff, fresh: Boolean = false) {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            val original = SessionDisplayRig()
            deliver(); fixture.connect()
            val first = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(first.get(), "session", original.session)
            val saved = Bundle()
            try {
                first.start()
                cutOff(fixture, cutoff)
                first.saveInstanceState(saved)
            } finally { first.stop().destroy() }
            if (fresh) deliver()
            checkRestored(saved, fresh)
        }
    }

    private fun checkRestored(saved: Bundle, fresh: Boolean) {
        val rig = SessionDisplayRig()
        val activity = Robolectric.buildActivity(MainActivity::class.java).create(saved)
        ReflectionHelpers.setField(activity.get(), "session", rig.session)
        try {
            activity.start()
            assertEquals("T1414 recreation is not protected delivery", if (fresh) 1 else 0, rig.sockets.size)
            if (!fresh) assertCutOff(rig)
        } finally {
            activity.stop().destroy(); CameraOwner.reset(); AudioOwner.reset()
            shadowOf(Looper.getMainLooper()).idle()
        }
    }

    private fun assertCutOff(rig: SessionDisplayRig) {
        assertNull("T1414 no ADB control authority", rig.control.routes)
        assertNull("T1414 no ADB video authority", rig.receiver.routes)
        assertNull(rig.job())
        assertTrue(rig.sockets.isEmpty())
    }

    @Test fun t1414FreshProcessCompositionRetainsPersistedCutoff() {
        val original = SessionDisplayRig()
        deliver()
        original.selectDisplay(original.endpoint)
        original.selectDisplay(null)
        original.session.stop()
        // New coordinator and empty process-owned USB state; only settings survive.
        val restored = SessionDisplayRig()
        try {
            ConnectionDisplayBinding(restored.session).apply(ConnectionView())
            restored.session.start()
            assertCutOff(restored)
        } finally { restored.session.stop(); shadowOf(Looper.getMainLooper()).idle() }
    }

    @Test fun t1414UntouchedAdbStartupStillConnects() {
        val rig = SessionDisplayRig()
        try {
            deliver()
            ConnectionDisplayBinding(rig.session).apply(ConnectionView())
            rig.session.start()
            assertEquals(1, rig.sockets.size)
            assertEquals(AdbDisplayRoutes.create().controlPort, rig.control.routes?.controlPort)
        } finally { rig.session.stop(); shadowOf(Looper.getMainLooper()).idle() }
    }
}
