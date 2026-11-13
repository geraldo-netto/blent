package com.blent

import android.content.Context
import android.content.Intent
import android.os.Looper
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AdbDeliveryCutoffTest {
    private fun deliver() = assertTrue(TokenDelivery.store(RuntimeEnvironment.getApplication(),
        Intent().putExtra("token", "a".repeat(64))))

    @Test fun t1414CutoffAndDeliveryRemainOrderedAcrossPreferenceOwners() {
        val app = RuntimeEnvironment.getApplication()
        val prefs = Prefs(app)
        assertTrue(prefs.adbDeliveryPermitted)
        prefs.cutOffAdbDelivery()
        assertFalse(Prefs(app).adbDeliveryPermitted)
        deliver()
        assertTrue(Prefs(app).adbDeliveryPermitted)
        Prefs(app).cutOffAdbDelivery()
        assertFalse(prefs.adbDeliveryPermitted)
        prefs.hostToken = "b".repeat(64)
        assertFalse("T1414 token equality/change is not protected delivery", prefs.adbDeliveryPermitted)
        deliver()
        assertTrue(prefs.adbDeliveryPermitted)
    }

    @Test fun t1414InvalidOrExhaustedPersistedCutoffsFailClosed() {
        val app = RuntimeEnvironment.getApplication()
        val storage = app.getSharedPreferences("blent", Context.MODE_PRIVATE)
        deliver()
        for (invalid in listOf(Long.MIN_VALUE, -1L, Long.MAX_VALUE)) {
            storage.edit().putLong("adb_delivery_cutoff", invalid).apply()
            val rig = SessionDisplayRig()
            try {
                rig.session.start()
                assertNull("T1414 invalid cutoff cannot select ADB", rig.control.routes)
                assertNull(rig.receiver.routes)
                assertTrue(rig.sockets.isEmpty())
            } finally { rig.session.stop() }
        }
        shadowOf(Looper.getMainLooper()).idle()
    }

    @Test fun t1414InvalidDeliveryCannotTurnCutoffIntoUntouchedStartup() {
        val app = RuntimeEnvironment.getApplication()
        val storage = app.getSharedPreferences("blent", Context.MODE_PRIVATE)
        for (invalid in listOf(Long.MIN_VALUE, -1L)) {
            storage.edit().putLong("host_token_delivery", invalid).apply()
            Prefs(app).cutOffAdbDelivery()
            assertFalse(Prefs(app).adbDeliveryPermitted)
        }
    }

    @Test fun t1414UntouchedStartupRetainsTokenDeliveryHandshake() {
        val rig = SessionDisplayRig()
        rig.prefs.hostToken = null
        try {
            rig.session.start()
            assertEquals(1, rig.sockets.size)
            assertNull(rig.control.token)
            assertNotNull(rig.control.routes)
        } finally { rig.session.stop(); shadowOf(Looper.getMainLooper()).idle() }
    }
}
