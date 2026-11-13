package com.blent

import android.content.Intent
import android.os.Looper
import com.blent.connection.*
import kotlinx.coroutines.*
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
class NullDisplayCutoffTest {
    private enum class Cutoff { DISCONNECT, DENY, REVOKE }
    private fun idle() { shadowOf(Looper.getMainLooper()).idle() }
    private fun deliver() = assertTrue(TokenDelivery.store(RuntimeEnvironment.getApplication(),
        Intent().putExtra("token", "a".repeat(64))))

    @Test fun t1420DisconnectWithoutUsbRouteRevokesPreviouslyAdmittedAdb() = cutOff(Cutoff.DISCONNECT)
    @Test fun t1420DenialWithoutUsbRouteRevokesPreviouslyAdmittedAdb() = cutOff(Cutoff.DENY)
    @Test fun t1420RevocationWithoutUsbRouteRevokesPreviouslyAdmittedAdb() = cutOff(Cutoff.REVOKE)

    private fun stop(fixture: ConnectionUiFixture, cutoff: Cutoff) {
        val choice = checkNotNull(fixture.binding.state.value.peer)
        when (cutoff) {
            Cutoff.DISCONNECT -> fixture.binding.disconnect()
            Cutoff.DENY -> assertTrue(fixture.binding.decide(choice, false, false))
            Cutoff.REVOKE -> assertTrue(fixture.binding.revoke(choice))
        }
        idle()
    }

    private fun restored(fixture: ConnectionUiFixture, rig: SessionDisplayRig) {
        deliver(); idle()
        assertEquals(8891, rig.control.routes?.controlPort)
        val restored = rig.sockets.last().first
        val count = rig.sockets.size
        fixture.workers.single().released.complete(Unit)
        idle()
        assertEquals("T1420 original retirement is not a new cutoff", count, rig.sockets.size)
        assertFalse(restored.cancelled)
        assertEquals(8891, rig.receiver.routes?.controlPort)
    }

    private fun close(rig: SessionDisplayRig) = runBlocking {
        rig.session.stop()
        val scope = ReflectionHelpers.getField<CoroutineScope>(rig.control.control, "scope")
        checkNotNull(scope.coroutineContext[Job]).cancelAndJoin()
        rig.drain(); idle()
    }

    private fun cutOff(cutoff: Cutoff) {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            val rig = SessionDisplayRig()
            deliver(); fixture.connect(); fixture.prompt()
            val activity = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(activity.get(), "session", rig.session)
            try {
                activity.start().resume(); idle()
                assertNull(rig.control.routes)
                deliver(); idle()
                assertEquals(8891, rig.control.routes?.controlPort)
                val admitted = rig.sockets.last().first
                stop(fixture, cutoff)
                assertFalse(rig.prefs.adbDeliveryPermitted)
                assertNull("T1420 null selection still cuts off control", rig.control.routes)
                assertNull("T1420 null selection still cuts off video", rig.receiver.routes)
                assertNull(rig.job())
                assertTrue(admitted.cancelled)
                restored(fixture, rig)
            } finally {
                activity.pause().stop().destroy(); close(rig)
                CameraOwner.reset(); AudioOwner.reset(); idle()
            }
        }
    }
}
