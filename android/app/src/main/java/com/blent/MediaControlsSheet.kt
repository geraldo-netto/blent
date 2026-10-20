package com.blent

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

internal enum class MainControls { SETTINGS, MEDIA }

/** Activity composition borrows the existing controls; opening this sheet creates
 * no media owner, grants no consent and retains no native resource or callback. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun MediaControlsSheet(controls: @Composable () -> Unit, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Column(Modifier.verticalScroll(rememberScrollState()).padding(24.dp)) {
            Text("Camera and audio", style = MaterialTheme.typography.titleLarge)
            TextButton(onClick = onDismiss) { Text("Close camera and audio controls") }
            controls()
        }
    }
}
