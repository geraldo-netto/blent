package com.blent

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun CameraControls(binding: CameraBinding) {
    val state by binding.state.collectAsState()
    SettingsSection("Camera sharing")
    Text(state.status, style = MaterialTheme.typography.bodySmall)
    Text("Configure cameras in the Cameras tab of Blent on your computer.",
        style = MaterialTheme.typography.bodySmall)
    Spacer(Modifier.height(8.dp))
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = { binding.choose(state.endpoint?.requestedLens) },
            enabled = state.selected == null && state.endpoint?.requestedLens != null) {
            Text("Start camera")
        }
        OutlinedButton(onClick = { binding.choose(null) }, enabled = state.selected != null) {
            Text("Stop camera")
        }
    }
    Text("Camera controls leave display sharing unchanged.", style = MaterialTheme.typography.bodySmall)
    Spacer(Modifier.height(20.dp))
}
