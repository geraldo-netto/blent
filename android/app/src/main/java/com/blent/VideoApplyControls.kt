package com.blent

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** Presentation borrows the sheet's sole draft; callbacks never retain media owners. */
@Composable
internal fun VideoApplyControls(
    change: StreamOverrides,
    hasOverrides: Boolean,
    onApply: (StreamOverrides) -> Unit,
    onUseHost: () -> Unit,
) {
    Text(if (change.empty) "No pending video edits" else "Video changes not applied",
        style = MaterialTheme.typography.bodyMedium)
    Button(onClick = { onApply(change) }, enabled = !change.empty, modifier = Modifier.fillMaxWidth()) {
        Text("Apply video changes")
    }
    if (hasOverrides) {
        TextButton(onClick = onUseHost) { Text("Use host settings") }
    }
    Text("Apply video changes sends only edited fields and remembers them for reconnects. " +
        "Use host settings clears these overrides; it does not undo changes already applied on the host. " +
        "Display sharing may briefly restart. Camera sharing keeps its current state.",
        style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    Spacer(Modifier.height(20.dp))
}

@Composable
internal fun DiscardVideoDialog(onKeepEditing: () -> Unit, onDiscard: () -> Unit) {
    AlertDialog(
        onDismissRequest = onKeepEditing,
        title = { Text("Discard video changes?") },
        text = { Text("Only unapplied video edits will be discarded. Display and app switches were applied immediately.") },
        confirmButton = { TextButton(onClick = onDiscard) { Text("Discard video changes") } },
        dismissButton = { TextButton(onClick = onKeepEditing) { Text("Keep editing") } },
    )
}
