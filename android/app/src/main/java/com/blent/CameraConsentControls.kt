package com.blent

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

/** The rendered offer is retained by each callback; authority validation belongs
 * to MediaAccessControls<CameraAccessOffer>, so a queued tap cannot grant a replacement owner. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun CameraConsentControls(access: MediaAccessControls<CameraAccessOffer>, view: MediaConsentView<CameraAccessOffer>) {
    val offer = view.offer ?: return
    Column {
        SettingsSection("Camera sharing")
        Text("USB camera request: ${if (offer.lens == CameraLens.FRONT) "Front" else "Rear"}.")
        Text(cameraConsentStatus(view), style = MaterialTheme.typography.bodySmall)
        Text("Choose lens and profile on your computer. Pairing does not grant camera permission.",
            style = MaterialTheme.typography.bodySmall)
        Spacer(Modifier.height(8.dp))
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { access.start(offer) }, enabled = !view.enabled && view.foreground) {
                Text("Start camera")
            }
            OutlinedButton(onClick = { access.stop(offer) }, enabled = view.enabled) { Text("Stop camera") }
        }
        Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
            Switch(checked = view.backgroundAllowed, onCheckedChange = { access.background(offer, it) },
                enabled = offer.backgroundRequested,
                modifier = Modifier.semantics { contentDescription = "Allow camera in background" })
            Text("Allow camera in background")
        }
        Text("Background capture needs both your choice and the computer's request; it keeps camera service and CPU awake.",
            style = MaterialTheme.typography.bodySmall)
        Text("Camera controls leave display sharing unchanged.", style = MaterialTheme.typography.bodySmall)
        Spacer(Modifier.height(20.dp))
    }
}

internal fun cameraConsentStatus(view: MediaConsentView<CameraAccessOffer>): String = when {
    !view.enabled -> "Camera is off. Start locally to accept this request."
    view.offer?.permissionRequired == true -> "Camera requested; waiting for Android permission."
    view.offer?.admitted == true -> "Camera admitted; capture follows local permission and lifecycle."
    else -> "Camera requested; waiting for admission."
}
