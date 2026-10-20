package com.blent.connection

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp

/** View disposal only revokes this ticket. The application owner retains any
 * accepted request; recreation never automatically starts or repeats a forget. */
@Composable
internal fun SavedComputerControls(binding: SavedTrustBinding) {
    var browser by remember(binding) { mutableStateOf<Any?>(null) }
    OutlinedButton(onClick = { browser = Any() }) { Text("Saved computers") }
    browser?.let { ticket ->
        key(ticket) {
            SavedComputerDialog(binding) { if (browser === ticket) browser = null }
        }
    }
}

@Composable
internal fun SavedComputerDialog(binding: SavedTrustBinding, dismiss: () -> Unit) {
    val view by binding.state.collectAsState()
    val use by binding.connectionUse.collectAsState(initial = null)
    val client = rememberSavedBrowserClient(binding)
    var choice by remember(view.request, client) { mutableStateOf<SavedTrustChoice?>(null) }
    val ownsView = client?.current == true
    val available = ownsView && use == null && view.phase !in setOf(SavedTrustPhase.BUSY, SavedTrustPhase.RETIRING, SavedTrustPhase.QUARANTINED)
    val close = {
        client?.close()
        choice = null
        dismiss()
    }
    AlertDialog(
        onDismissRequest = close,
        title = { Text("Saved computers") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                SavedComputerStatus(view, use)
                if (client != null && !ownsView) Text("This saved-computer view is no longer current. Close and reopen it.")
                SavedComputerList(view, available) { peer ->
                    choice = client?.choice(peer)
                }
            }
        },
        confirmButton = {
            TextButton(onClick = { client?.refresh() }, enabled = available) {
                Text("Refresh saved computers")
            }
        },
        dismissButton = {
            Row {
                if (view.phase == SavedTrustPhase.BUSY) {
                    TextButton(onClick = { client?.stop() }, enabled = ownsView) {
                        Text("Stop saved-computer access")
                    }
                }
                TextButton(onClick = close) { Text("Close saved computers") }
            }
        },
    )
    choice?.let { selected ->
        SavedComputerConfirmation(selected, available,
            {
                if (choice === selected) {
                    choice = null
                    client?.forget(selected)
                }
            },
            { if (choice === selected) choice = null })
    }
}

/** Composition owns the browser ticket; accepted work stays application-owned. */
@Composable
private fun rememberSavedBrowserClient(binding: SavedTrustBinding): SavedTrustClient? {
    var client by remember(binding) { mutableStateOf<SavedTrustClient?>(null) }
    DisposableEffect(binding) {
        val attached = binding.attach()
        client = attached
        if (binding.state.value.phase == SavedTrustPhase.IDLE) attached.refresh()
        object : DisposableEffectResult {
            override fun dispose() {
                attached.close()
                client = null
            }
        }
    }
    return client
}

@Composable
internal fun SavedComputerStatus(view: SavedTrustView, use: ConnectionUse?) {
    val status = when (view.phase) {
        SavedTrustPhase.IDLE -> "Refresh to load saved computers."
        SavedTrustPhase.BUSY -> "Reading or updating saved computers…"
        SavedTrustPhase.RETIRING -> "Waiting for saved-computer access to close…"
        SavedTrustPhase.READY -> if (view.computers?.peers.isNullOrEmpty()) "No saved computers." else "Saved computers are ready."
        SavedTrustPhase.FAILED -> "Saved-computer access failed."
        SavedTrustPhase.QUARANTINED -> "Saved-computer cleanup is unconfirmed; access remains blocked."
    }
    Text(status)
    if (use == ConnectionUse.CONNECTION) Text("Disconnect USB and wait for cleanup before refreshing or forgetting.")
    view.failure?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    view.cleanup?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
}

/** Lazy-list callbacks borrow this one immutable bounded snapshot. Framework
 * indices are in 0 until its captured count; no mutable replacement is indexed. */
@Composable
internal fun SavedComputerList(view: SavedTrustView, available: Boolean, select: (ConnectionPeer) -> Unit) {
    if (view.phase != SavedTrustPhase.READY) return
    val peers = view.computers?.peers ?: emptyList()
    LazyColumn(Modifier.heightIn(max = 280.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        items(count = peers.size, key = { index -> peers[index].fingerprint }) { index ->
            SavedComputerRow(peers[index], available, select)
        }
    }
}

@Composable
internal fun SavedComputerRow(peer: ConnectionPeer, available: Boolean, select: (ConnectionPeer) -> Unit) {
    Column {
        SelectionContainer {
            Text("Computer fingerprint: ${peer.fingerprint}", fontFamily = FontFamily.Monospace)
        }
        OutlinedButton(onClick = { select(peer) }, enabled = available) {
            Text("Forget ${peer.fingerprint}")
        }
    }
}

@Composable
internal fun SavedComputerConfirmation(choice: SavedTrustChoice, available: Boolean, forget: () -> Unit, dismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text("Forget this computer?") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                SelectionContainer { Text(choice.selected.peer.fingerprint, fontFamily = FontFamily.Monospace) }
                Text("This removes saved trust. The next connection will require approval. Camera and microphone permissions are separate.")
            }
        },
        confirmButton = {
            TextButton(onClick = forget, enabled = available) { Text("Confirm forget computer") }
        },
        dismissButton = {
            TextButton(onClick = dismiss) { Text("Keep computer") }
        },
    )
}
