package com.blent

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

/** Render callbacks retain the original offer. Persisted settings are additional
 * restrictions; they never grant fresh per-offer Start/background consent. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun AudioConsentControls(binding: AudioBinding, access: MediaAccessControls<AudioAccessOffer>, view: MediaConsentView<AudioAccessOffer>) {
    val offer = view.offer ?: return
    val state by binding.state.collectAsState()
    val canStart = !offer.backgroundRequested || state.preferences.background
    Column {
        SettingsSection("${binding.label.replaceFirstChar { it.uppercase() }} sharing")
        Text("USB ${binding.label} request.")
        Text(audioConsentStatus(view), style = MaterialTheme.typography.bodySmall)
        Text("Pairing does not grant audio access. Changing tablet settings stops the session.", style = MaterialTheme.typography.bodySmall)
        if (!canStart) Text("This request needs background ${binding.label}. Enable the background setting below first.", style = MaterialTheme.typography.bodySmall)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { if (!offer.backgroundRequested || binding.preferences.background) access.start(offer) },
                enabled = !view.enabled && view.foreground && canStart) { Text("Start ${binding.label}") }
            OutlinedButton(onClick = { access.stop(offer) }, enabled = view.enabled) { Text("Stop ${binding.label}") }
        }
        Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
            Switch(checked = view.backgroundAllowed, onCheckedChange = { access.background(offer, it) },
                enabled = offer.backgroundRequested,
                modifier = Modifier.semantics { contentDescription = "Allow this ${binding.label} request in background" })
            Text("Allow this request in background")
        }
        Text("Background sharing needs both your choice and the computer's request; it keeps audio service and CPU awake.", style = MaterialTheme.typography.bodySmall)
        Text("Audio controls leave display and camera sharing unchanged.", style = MaterialTheme.typography.bodySmall)
        Spacer(Modifier.height(8.dp))
    }
}
internal fun audioConsentStatus(view: MediaConsentView<AudioAccessOffer>): String = when {
    !view.enabled -> "Audio is off. Start locally to accept this request."
    view.offer?.permissionRequired == true -> "Audio requested; waiting for Android permission."
    view.offer?.admitted == true -> "Audio admitted; playback/capture follows local permission and lifecycle."
    else -> "Audio requested; waiting for admission."
}
