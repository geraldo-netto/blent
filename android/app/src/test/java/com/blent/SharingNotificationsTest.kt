package com.blent

import android.Manifest
import android.app.NotificationManager
import android.content.pm.PackageManager
import android.os.Build
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class SharingNotificationsTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Suppress("DEPRECATION")
    @Test fun t1571_permissionResultAndSettingsPreserveLocalStopGuidance() {
        val app = RuntimeEnvironment.getApplication()
        shadowOf(app).denyPermissions(Manifest.permission.POST_NOTIFICATIONS)
        shadowOf(app.getSystemService(NotificationManager::class.java)).setNotificationsEnabled(false)
        compose.setContent { BlentTheme { SharingNotifications() } }
        compose.onNodeWithText("Notifications are disabled.", substring = true).assertExists()
        if (Build.VERSION.SDK_INT >= 33) requestNotificationPermission(false)
        else compose.onNodeWithText("Allow notifications").assertDoesNotExist()
        compose.onNodeWithText("Notification settings").performClick()
        val settings = shadowOf(compose.activity).nextStartedActivity
        assertEquals(Settings.ACTION_APP_NOTIFICATION_SETTINGS, settings.action)
        assertEquals(app.packageName, settings.getStringExtra(Settings.EXTRA_APP_PACKAGE))
    }

    @Test fun t1571_grantMakesNotificationStopAvailable() {
        val app = RuntimeEnvironment.getApplication()
        shadowOf(app).denyPermissions(Manifest.permission.POST_NOTIFICATIONS)
        shadowOf(app.getSystemService(NotificationManager::class.java)).setNotificationsEnabled(false)
        compose.setContent { BlentTheme { SharingNotifications() } }
        if (Build.VERSION.SDK_INT >= 33) requestNotificationPermission(true)
        else compose.onNodeWithText("Allow notifications").assertDoesNotExist()
    }

    @Suppress("DEPRECATION")
    private fun requestNotificationPermission(granted: Boolean) {
        compose.onNodeWithText("Allow notifications").performClick()
        val request = shadowOf(compose.activity).nextStartedActivityForResult
        assertNotNull(request)
        assertTrue("T1571 notification action did not request permission", request.requestCode >= 0)
        val app = RuntimeEnvironment.getApplication()
        compose.runOnIdle {
            if (granted) shadowOf(app).grantPermissions(Manifest.permission.POST_NOTIFICATIONS)
            shadowOf(app.getSystemService(NotificationManager::class.java)).setNotificationsEnabled(granted)
            if (granted) assertTrue("T1571 native permission probe remains denied", sharingNotificationsEnabled(compose.activity))
            compose.activity.onRequestPermissionsResult(request.requestCode,
                arrayOf(Manifest.permission.POST_NOTIFICATIONS),
                intArrayOf(if (granted) PackageManager.PERMISSION_GRANTED else PackageManager.PERMISSION_DENIED))
        }
        val text = if (granted) "Notifications can provide Stop" else "Notifications are disabled."
        compose.onNodeWithText(text, substring = true).assertExists()
    }

    @Test fun t1571_enabledNotificationsAndOsDisableAreObservedOnResume() {
        val app = RuntimeEnvironment.getApplication()
        shadowOf(app).grantPermissions(Manifest.permission.POST_NOTIFICATIONS)
        val manager = shadowOf(app.getSystemService(NotificationManager::class.java))
        manager.setNotificationsEnabled(true)
        compose.setContent { BlentTheme { SharingNotifications() } }
        compose.onNodeWithText("Notifications can provide Stop", substring = true).assertExists()
        compose.runOnIdle {
            manager.setNotificationsEnabled(false)
            val lifecycle = compose.activity.lifecycle as androidx.lifecycle.LifecycleRegistry
            lifecycle.handleLifecycleEvent(androidx.lifecycle.Lifecycle.Event.ON_PAUSE)
            lifecycle.handleLifecycleEvent(androidx.lifecycle.Lifecycle.Event.ON_RESUME)
        }
        compose.onNodeWithText("Notifications are disabled.", substring = true).assertExists()
    }
}
