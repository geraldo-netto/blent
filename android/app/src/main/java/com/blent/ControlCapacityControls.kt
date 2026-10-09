package com.blent

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun ControlCapacityControls(bytes: Int, change: (Int) -> Unit) {
    Text("Control queue limit: ${bytes / 1024} KiB")
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (kib in listOf(8, 64, 256, 1024)) {
            FilterChip(selected = bytes == kib * 1024, onClick = { change(kib * 1024) }, label = { Text("$kib KiB") })
        }
    }
    Text("Smaller limits reconnect sooner when input backs up. Larger limits tolerate bursts but may delay input. Applies to new sends, including decoder reports.",
        style = MaterialTheme.typography.bodySmall)
}
