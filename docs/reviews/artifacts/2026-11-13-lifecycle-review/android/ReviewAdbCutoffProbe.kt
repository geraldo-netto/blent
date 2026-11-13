package com.blent

import android.content.Intent
import com.blent.connection.ConnectionDisplayBinding
import com.blent.connection.ConnectionUiFixture
import org.junit.Assert.*
import org.junit.Test
import org.junit.After
import android.os.Looper
import org.robolectric.Shadows.shadowOf
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

/** Hosted composition probe, not a permanent regression or device execution. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
@LooperMode(LooperMode.Mode.PAUSED)
class ReviewAdbCutoffProbe {
    @After fun drainUi() { shadowOf(Looper.getMainLooper()).idle() }
    @Test fun replacementActivityCompositionMustRetainAdbCutoff() {
        recreate(false)
    }

    @Test fun freshProtectedDeliveryPermitsReplacementActivityAdb() {
        recreate(true)
    }

    private fun recreate(fresh: Boolean) {
        ConnectionUiFixture().use { fixture ->
            val original = SessionDisplayRig()
            assertTrue(deliver())
            val routes = ConnectionDisplayBinding(original.session)
            fixture.connect()
            routes.apply(fixture.binding.state.value)
            fixture.binding.disconnect()
            fixture.workers.single().released.complete(Unit)
            routes.apply(fixture.binding.state.value)
            assertNull(original.control.routes)
            original.session.stop()
            if (fresh) assertTrue(deliver())
            val restored = SessionDisplayRig()
            try {
                ConnectionDisplayBinding(restored.session).apply(fixture.binding.state.value)
                restored.session.start()
                println("CUTOFF fresh=$fresh active=${fixture.binding.state.value.active} reconnect=${fixture.binding.state.value.reconnectAllowed} delivery=${restored.prefs.hostTokenDelivery} sockets=${restored.sockets.size} port=${restored.control.routes?.controlPort}")
                assertEquals("Only post-cutoff protected delivery authorizes ADB after recreation", if (fresh) 1 else 0, restored.sockets.size)
            } finally { restored.session.stop() }
        }
    }

    @Test fun originalActivityCompositionKeepsAdbCutoff() {
        val rig = SessionDisplayRig()
        try {
            assertTrue(deliver())
            rig.session.selectDisplay(rig.endpoint)
            rig.session.selectDisplay(null)
            rig.session.stop()
            rig.session.start()
            assertTrue(rig.sockets.isEmpty())
        } finally { rig.session.stop() }
    }

    private fun deliver(): Boolean = TokenDelivery.store(RuntimeEnvironment.getApplication(),
        Intent().putExtra("token", "a".repeat(64)))
}
