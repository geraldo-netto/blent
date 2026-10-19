package com.blent.connection

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties
import kotlinx.coroutines.launch

private data class PreferenceDescription(val field: ConnectionPreferenceField, val title: String, val help: String)
private val preferenceDescriptions = listOf(
    PreferenceDescription(ConnectionPreferenceField.CHANNEL_BYTES, "Channel buffer (bytes)", "4096–1048576. Larger windows retain more buffered data; both sides limit the negotiated size."),
    PreferenceDescription(ConnectionPreferenceField.QUEUED_MESSAGES, "Queued messages", "1–1024. Larger queues use more memory and can increase waiting time."),
    PreferenceDescription(ConnectionPreferenceField.QUANTUM, "Transfer quantum (bytes)", "1–65536. Smaller chunks favor interleaving with more dispatch work."),
    PreferenceDescription(ConnectionPreferenceField.RESOURCE_TIMEOUT, "Resource request timeout (ms)", "1–600000. Longer waits retain pending resource requests longer."),
    PreferenceDescription(ConnectionPreferenceField.IO_TIMEOUT, "I/O timeout (ms)", "1–60000. Shorter budgets can reject slow operations."),
    PreferenceDescription(ConnectionPreferenceField.RETIREMENT_TIMEOUT, "Cleanup observation timeout (ms)", "1–60000. Expiry never proves a resource has stopped or allows unsafe replacement."),
    PreferenceDescription(ConnectionPreferenceField.APPROVAL_TIMEOUT, "Approval timeout (ms)", "1–600000. Time allowed for local pairing or permission approval."),
    PreferenceDescription(ConnectionPreferenceField.NATIVE_TIMEOUT, "Native setup timeout (ms)", "1–60000. Time allowed for native opening and setup."),
    PreferenceDescription(ConnectionPreferenceField.TRUSTED_PEERS, "Saved computer capacity", "1–1024. A limit below the saved count rejects setup; it does not erase trust."),
    PreferenceDescription(ConnectionPreferenceField.ACCESSORY_CAPACITY, "Accessory inventory capacity", "1–1024. Higher limits allow more accessory metadata."),
    PreferenceDescription(ConnectionPreferenceField.FALLBACK, "Idle fallback (ms)", "1–1000. Longer intervals reduce polling but delay missed hints. Earlier deadlines still win; unknown owners and cleanup retain a 10 ms maximum."),
)

@Composable
internal fun ConnectionPreferenceDialog(store: ConnectionPreferenceStore, dismiss: () -> Unit) {
    val form = remember(store) { ConnectionPreferenceForm() }
    val scope = rememberCoroutineScope()
    LaunchedEffect(form) { form.load(store) }
    AlertDialog(
        // Keep content width stable across the platform's intrinsic width probes.
        modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(0.9f),
        properties = DialogProperties(usePlatformDefaultWidth = false),
        onDismissRequest = dismiss,
        title = { Text("USB connection settings") },
        text = { Column(Modifier.heightIn(max = 400.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Saved values apply to the next USB connection. Active streams keep their current settings.")
            if (form.busy) Text("Waiting for USB settings storage…")
            form.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            form.notice?.let { Text(it) }
            for (description in preferenceDescriptions) PreferenceInput(form, description)
            OutlinedButton(onClick = form::defaults, enabled = !form.busy) { Text("Use defaults") }
            OutlinedButton(onClick = { scope.launch { form.load(store) } }, enabled = !form.busy) { Text("Reload saved settings") }
        } },
        confirmButton = { TextButton(onClick = { scope.launch { form.save(store) } }, enabled = !form.busy) { Text("Save USB settings") } },
        dismissButton = { TextButton(onClick = dismiss) { Text("Close settings") } },
    )
}

@Composable
private fun PreferenceInput(form: ConnectionPreferenceForm, description: PreferenceDescription) {
    OutlinedTextField(
        value = form.fields.getValue(description.field.key),
        onValueChange = { form.edit(description.field, it) },
        label = { Text(description.title) },
        supportingText = { Text(description.help) },
        isError = form.invalid(description.field), enabled = !form.busy, singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
    )
}
