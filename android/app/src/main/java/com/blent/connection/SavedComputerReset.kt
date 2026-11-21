package com.blent.connection

import androidx.compose.material3.*
import androidx.compose.runtime.*

/** A confirmation belongs to this viewer and request. Disposal revokes the
 * client; accepted storage work remains in the existing private-process lane. */
@Composable
internal fun SavedComputerReset(client: SavedTrustClient?, request: Any?, available: Boolean) {
    var confirmation by remember(client, request) { mutableStateOf<Any?>(null) }
    TextButton(onClick = { confirmation = Any() }, enabled = available) { Text("Reset saved computers") }
    confirmation?.let { token ->
        SavedComputerResetConfirmation(available,
            {
                if (confirmation === token) {
                    confirmation = null
                    client?.reset(request)
                }
            },
            { if (confirmation === token) confirmation = null })
    }
}

@Composable
private fun SavedComputerResetConfirmation(available: Boolean, reset: () -> Unit, dismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text("Reset all saved computers?") },
        text = { Text("This removes all remembered computer approvals and repairs interrupted trust updates. Your tablet identity stays the same. Each computer will require approval again.") },
        confirmButton = { TextButton(onClick = reset, enabled = available) { Text("Confirm reset saved computers") } },
        dismissButton = { TextButton(onClick = dismiss) { Text("Keep saved computers") } },
    )
}
