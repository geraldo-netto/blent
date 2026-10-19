package com.blent.connection

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.blent.SettingsSection

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun ConnectionControls(binding: ConnectionBinding, configured: ConfiguredConnection) {
    val view by binding.state.collectAsState()
    val configuration by configured.state.collectAsState()
    var settings by remember { mutableStateOf(false) }
    SettingsSection("USB connection")
    Text(view.status)
    view.cleanup?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
    if (configuration.pending) Text("Loading USB connection settings…")
    configuration.failure?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = configured::connect, enabled = !view.active && !configuration.pending) { Text("Connect USB") }
        OutlinedButton(onClick = configured::disconnect, enabled = view.active || configuration.pending) { Text("Disconnect USB") }
        OutlinedButton(onClick = { settings = true }) { Text("USB capacity and deadlines") }
    }
    if (settings) ConnectionPreferenceDialog(configured.preferences) { settings = false }
    view.peer?.let { selected ->
        ConnectionFingerprints(selected)
        Text(if (view.remembered == true) "Computer trust saved." else "Computer trust is not saved.")
        OutlinedButton(onClick = { binding.revoke(selected) }, enabled = view.active) { Text("Revoke computer") }
    }
    Text("Pairing does not enable camera or microphone sharing.", style = MaterialTheme.typography.bodySmall)
    Spacer(Modifier.height(20.dp))
}

@Composable
internal fun ConnectionFingerprints(choice: ConnectionChoice) {
    SelectionContainer {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Computer fingerprint: ${choice.ticket.peer.fingerprint}", fontFamily = FontFamily.Monospace)
            Text("Tablet fingerprint: ${choice.tablet.fingerprint}", fontFamily = FontFamily.Monospace)
        }
    }
}

/** Always visible while approval is pending, even with the settings sheet closed.
 * Local checkbox resets for every exact choice; no remember or approve on attach. */
@Composable
internal fun ConnectionPairingDialog(binding: ConnectionBinding) {
    val view by binding.state.collectAsState()
    val choice = view.approval ?: return
    var rememberComputer by remember(choice) { mutableStateOf(false) }
    AlertDialog(
        onDismissRequest = { binding.decide(choice, false, false) },
        title = { Text("Pair with this computer?") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Compare Computer with Computer and Tablet with Tablet on both devices. Approve only when both labels match.")
                ConnectionFingerprints(choice)
                Row {
                    Checkbox(checked = rememberComputer, onCheckedChange = { rememberComputer = it },
                        modifier = Modifier.semantics { contentDescription = "Remember this computer" })
                    Text("Remember this computer")
                }
                Text("Camera and microphone sharing require separate permission.")
            }
        },
        confirmButton = { TextButton(onClick = { binding.decide(choice, true, rememberComputer) }) { Text("Approve computer") } },
        dismissButton = { TextButton(onClick = { binding.decide(choice, false, false) }) { Text("Deny") } },
    )
}
