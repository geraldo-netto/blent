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
class BackgroundAdbDeliveryTest {
    private enum class Cutoff { DISCONNECT, DENY, REVOKE }
    private fun idle() { shadowOf(Looper.getMainLooper()).idle() }
    private fun deliver() = assertTrue(TokenDelivery.store(RuntimeEnvironment.getApplication(),
        Intent().putExtra("token", "a".repeat(64))))

    @Test fun t1419BackgroundDisconnectKeepsLaterProtectedDelivery() = background(Cutoff.DISCONNECT, true)
    @Test fun t1419BackgroundDenialKeepsLaterProtectedDelivery() = background(Cutoff.DENY, true)
    @Test fun t1419BackgroundRevocationKeepsLaterProtectedDelivery() = background(Cutoff.REVOKE, true)
    @Test fun t1419BackgroundDisconnectRejectsEarlierDelivery() = background(Cutoff.DISCONNECT, false)
    @Test fun t1419BackgroundDenialRejectsEarlierDelivery() = background(Cutoff.DENY, false)
    @Test fun t1419BackgroundRevocationRejectsEarlierDelivery() = background(Cutoff.REVOKE, false)

    @Test fun t1419DelayedCutoffObservationReconcilesAlreadyDeliveredFreshCredentials() {
        val rig = SessionDisplayRig()
        try {
            deliver()
            rig.selectDisplay(rig.endpoint)
            rig.session.start()
            rig.prefs.cutOffAdbDelivery()
            deliver(); idle()
            assertTrue(rig.prefs.adbDeliveryPermitted)
            // The source cutoff preceded delivery; the UI projection arrives later.
            rig.session.selectDisplay(null)
            assertEquals(8891, rig.control.routes?.controlPort)
            assertEquals(rig.control.routes, rig.receiver.routes)
        } finally { close(rig) }
    }

    private fun select(fixture: ConnectionUiFixture, cutoff: Cutoff) {
        fixture.connect(); fixture.prompt()
        if (cutoff == Cutoff.DENY) return
        val worker = fixture.workers.single()
        worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        worker.report(ConnectionWorkerEvent.Display(displayRoute()))
    }
    private fun cutOff(fixture: ConnectionUiFixture, cutoff: Cutoff) {
        val choice = checkNotNull(fixture.binding.state.value.peer)
        when (cutoff) {
            Cutoff.DISCONNECT -> fixture.binding.disconnect()
            Cutoff.DENY -> assertTrue(fixture.binding.decide(choice, false, false))
            Cutoff.REVOKE -> assertTrue(fixture.binding.revoke(choice))
        }
        fixture.workers.single().released.complete(Unit)
        idle()
    }

    private fun close(rig: SessionDisplayRig) = runBlocking {
        rig.session.stop()
        val scope = ReflectionHelpers.getField<CoroutineScope>(rig.control.control, "scope")
        checkNotNull(scope.coroutineContext[Job]).cancelAndJoin()
        rig.drain(); idle()
    }

    private fun background(cutoff: Cutoff, fresh: Boolean) {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            val rig = SessionDisplayRig()
            deliver(); select(fixture, cutoff)
            val activity = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(activity.get(), "session", rig.session)
            try {
                activity.start().resume(); idle()
                activity.pause().stop()
                if (!fresh) deliver()
                cutOff(fixture, cutoff)
                if (fresh) deliver()
                idle()
                assertEquals("T1419 cutoff is recorded without an Activity observer", fresh, rig.prefs.adbDeliveryPermitted)
                repeat(2) {
                    activity.start().resume(); idle()
                    assertEquals("T1419 resume cannot change delivery order", fresh, rig.prefs.adbDeliveryPermitted)
                    assertEquals(if (fresh) 8891 else null, rig.control.routes?.controlPort)
                    activity.pause().stop()
                }
            } finally {
                activity.destroy(); close(rig); CameraOwner.reset(); AudioOwner.reset(); idle()
            }
        }
    }
}
