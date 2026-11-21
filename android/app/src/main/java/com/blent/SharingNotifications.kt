package com.blent

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.*
import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver

/** Composition owns the permission launcher and resume observer. No media
 * authority is retained by the result callback; denial leaves local Stop intact. */
@Composable
internal fun SharingNotifications() {
    val context = LocalContext.current
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    var enabled by remember(context) { mutableStateOf(sharingNotificationsEnabled(context)) }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
        enabled = sharingNotificationsEnabled(context)
    }
    DisposableEffect(context, lifecycle) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) enabled = sharingNotificationsEnabled(context)
        }
        lifecycle.addObserver(observer)
        onDispose { lifecycle.removeObserver(observer) }
    }
    Column {
        Text("Sharing notifications")
        Text(if (enabled) "Notifications can provide Stop while sharing in the background. Keep the sharing channel enabled."
            else "Notifications are disabled. Reopen Blent to stop sharing, or enable notifications for a Stop action.",
            style = MaterialTheme.typography.bodySmall)
        if (Build.VERSION.SDK_INT >= 33 && !enabled) {
            OutlinedButton(onClick = { permission.launch(Manifest.permission.POST_NOTIFICATIONS) }) { Text("Allow notifications") }
        }
        OutlinedButton(onClick = { openSharingNotificationSettings(context) }) { Text("Notification settings") }
    }
}

internal fun sharingNotificationsEnabled(context: Context): Boolean {
    if (Build.VERSION.SDK_INT >= 33 && context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) return false
    return context.getSystemService(NotificationManager::class.java).areNotificationsEnabled()
}

internal fun openSharingNotificationSettings(context: Context) {
    context.startActivity(Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
        .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
}
