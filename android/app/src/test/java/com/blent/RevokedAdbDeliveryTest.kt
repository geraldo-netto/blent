package com.blent

import android.content.Intent
import android.os.Looper
import com.blent.connection.ConnectionUiFixture
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
class RevokedAdbDeliveryTest {
    private fun deliver(token: String = "a".repeat(64)): Boolean =
        TokenDelivery.store(RuntimeEnvironment.getApplication(), Intent().putExtra("token", token))

    @Test fun t1404LauncherCannotRestoreRevokedDisplayFromCachedToken() {
        ConnectionUiFixture().use {
            WorkerTestBridge.install()
            val rig = SessionDisplayRig()
            val activity = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(activity.get(), "session", rig.session)
            try {
                assertTrue(deliver())
                rig.selectDisplay(rig.endpoint)
                rig.selectDisplay(null)
                activity.newIntent(Intent(Intent.ACTION_MAIN))
                assertNull("T1404 launcher is not credential delivery", rig.control.routes)
                assertNull(rig.receiver.routes)
                assertTrue(deliver()) // Same credentials, but a new protected delivery.
                activity.newIntent(Intent(Intent.ACTION_MAIN))
                assertEquals(AdbDisplayRoutes.create().controlPort, rig.control.routes?.controlPort)
            } finally {
                rig.session.stop(); activity.destroy(); CameraOwner.reset(); AudioOwner.reset()
            }
        }
    }

    @Test fun t1404OnlyPostCutoffDeliveryRestoresRoutesIncludingBackgroundDelivery() {
        val rig = SessionDisplayRig()
        try {
            assertTrue(deliver())
            rig.selectDisplay(rig.endpoint)
            assertTrue(deliver("b".repeat(64))) // Superseded by the later cutoff.
            rig.selectDisplay(null)
            rig.session.applyToken(true)
            assertNull("T1404 pre-cutoff delivery cannot restore routes", rig.control.routes)
            assertFalse(deliver("invalid"))
            rig.session.applyToken(true)
            assertNull(rig.control.routes)
            assertTrue(deliver())
            shadowOf(Looper.getMainLooper()).idle()
            assertTrue(rig.sockets.isEmpty())
            rig.session.start()
            assertEquals(AdbDisplayRoutes.create().controlPort, rig.control.routes?.controlPort)
            assertEquals(1, rig.sockets.size)
        } finally { rig.session.stop() }
    }

    @Test fun t1404DeliveryIdentityRejectsExhaustionAndInvalidCounters() {
        val app = RuntimeEnvironment.getApplication()
        val prefs = Prefs(app)
        val storage = app.getSharedPreferences("blent", android.content.Context.MODE_PRIVATE)
        assertTrue(deliver())
        for (invalid in listOf(-1L, Long.MAX_VALUE)) {
            storage.edit().putLong("host_token_delivery", invalid).apply()
            assertFalse(deliver("b".repeat(64)))
            assertEquals("a".repeat(64), prefs.hostToken)
            assertEquals(invalid, prefs.hostTokenDelivery)
        }
    }

    @Test fun t1404SameTokenProtectedDeliveryNotifiesForegroundSession() {
        val rig = SessionDisplayRig()
        try {
            assertTrue(deliver())
            rig.session.start()
            rig.selectDisplay(rig.endpoint)
            rig.selectDisplay(null)
            assertTrue(deliver())
            shadowOf(Looper.getMainLooper()).idle()
            assertEquals(AdbDisplayRoutes.create().controlPort, rig.control.routes?.controlPort)
        } finally { rig.session.stop() }
    }
}
