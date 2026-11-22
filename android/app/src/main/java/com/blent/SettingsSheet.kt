package com.blent

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.roundToInt

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun SettingsSheet(
    settings: SettingsValues,
    updateAvailable: String?,
    onOpenUpdate: () -> Unit,
    penOnly: Boolean,
    onDismiss: () -> Unit,
    displayRefreshRates: List<Float> = listOf(Prefs.DEFAULT_DISPLAY_REFRESH_RATE),
    onSettingsEvent: (SettingsEvent) -> Unit = {},
    cameraControls: @Composable () -> Unit = {},
    connectionControls: @Composable () -> Unit = {},
    decoderDiagnostics: DecoderDiagnostics = DecoderDiagnostics(),
) {
    var edited by remember { mutableStateOf(StreamOverrides()) }
    val bitrateMbps = (edited.bitrateKbps ?: settings.bitrateKbps) / 1000f
    val fpsChoice = edited.fps ?: settings.fps

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        containerColor = MaterialTheme.colorScheme.surface
    ) {
        Column(modifier = Modifier.verticalScroll(rememberScrollState()).padding(horizontal = 24.dp, vertical = 8.dp)) {
            Text(
                "Settings",
                fontSize = 20.sp,
                fontWeight = FontWeight.Bold,
                color = Color.White
            )
            Spacer(Modifier.height(20.dp))

            UpdateNotice(updateAvailable, onOpenUpdate)
            settings.streamError?.let { Text("Settings rejected: $it", color = MaterialTheme.colorScheme.error) }

            SettingsSection("Display & input")
            Text("Display and app switches apply immediately.", style = MaterialTheme.typography.bodySmall)
            Spacer(Modifier.height(8.dp))
            DisplayControls(settings, displayRefreshRates, onSettingsEvent)
            Spacer(Modifier.height(20.dp))

            ModeControl(penOnly) { onSettingsEvent(SettingsEvent.Mode(it)); onDismiss() }

            OrientationControls(settings.orientation) { onSettingsEvent(SettingsEvent.Orientation(it)) }

            if (!penOnly) {
                SettingsSection("Video stream")
                StreamControls(bitrateMbps, fpsChoice,
                    { edited = edited.copy(bitrateKbps = (it * 1000).roundToInt()) },
                    { edited = edited.copy(fps = it) })
            }

            connectionControls()
            cameraControls()

            SettingsSection("App")
            SettingsSwitch("Battery saver", "Reduce background work while keeping your brightness and frame-rate settings.", settings.batterySaver) { onSettingsEvent(SettingsEvent.BatterySaver(it)) }
            Spacer(Modifier.height(16.dp))
            AdvancedSettings("App & diagnostics") {
                SettingsSwitch("Show stats overlay", "FPS and bandwidth in the corner", settings.showStats) { onSettingsEvent(SettingsEvent.ShowStats(it)) }
                Spacer(Modifier.height(16.dp))
                SettingsSwitch("Check for newer releases", "One request to GitHub when the app opens. Nothing installs itself.", settings.checkUpdates) { onSettingsEvent(SettingsEvent.CheckUpdates(it)) }
                Spacer(Modifier.height(16.dp))
                ControlCapacityControls(settings.controlQueueBytes) { onSettingsEvent(SettingsEvent.ControlQueue(it)) }
                Spacer(Modifier.height(16.dp))
                DecoderDiagnosticsContent(decoderDiagnostics)
            }
            Spacer(Modifier.height(24.dp))

            if (!penOnly) {
                Button(
                    onClick = {
                        val change = edited.differentFrom(settings.bitrateKbps, settings.fps)
                        if (!change.empty) onSettingsEvent(SettingsEvent.Stream(change.bitrateKbps, change.fps))
                        onDismiss()
                    },
                    modifier = Modifier.fillMaxWidth(),
                    colors = ButtonDefaults.buttonColors(containerColor = Accent)
                ) {
                    Text("Apply", fontSize = 16.sp, modifier = Modifier.padding(vertical = 4.dp))
                }
                if (!settings.streamOverrides.empty) {
                    TextButton(onClick = { edited = StreamOverrides(); onSettingsEvent(SettingsEvent.ClearStreamOverrides) }) {
                        Text("Use host settings")
                    }
                }
                Spacer(Modifier.height(8.dp))
                Text(
                    "Apply sends only edited fields and remembers them for reconnects. Use host settings clears these overrides; it does not undo changes already applied on the host. Display sharing may briefly restart. Camera sharing keeps its current state.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.fillMaxWidth()
                )
            }
            Spacer(Modifier.height(24.dp))
        }
    }
}

@Composable
internal fun SettingsSection(title: String) {
    HorizontalDivider(color = Color(0xFF343444), modifier = Modifier.padding(bottom = 12.dp))
    Text(title, style = MaterialTheme.typography.titleMedium, color = Color.White,
        modifier = Modifier.semantics { heading() })
    Spacer(Modifier.height(12.dp))
}

@Composable
private fun AdvancedSettings(title: String, content: @Composable () -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    TextButton(onClick = { expanded = !expanded }, modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)
        .semantics { stateDescription = if (expanded) "Expanded" else "Collapsed" }) {
        Text(title, modifier = Modifier.weight(1f), textAlign = TextAlign.Start)
        Text(if (expanded) "Hide" else "Show")
    }
    if (expanded) content()
}

@Composable
private fun OrientationControls(orientation: Int, onOrientationChange: (Int) -> Unit) {
    // Which way round the tablet is held. Applies at once, in both
    // modes, and needs no Apply: it is the tablet's own business, the
    // host never sees it. Automatic uses the tilt sensor directly;
    // with it off, the two pinned directions appear.
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text("Rotate automatically", fontSize = 14.sp, color = Color(0xFFB0B0C0))
            Text(
                "Follow the tilt sensor between the two landscape directions",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        Spacer(Modifier.width(12.dp))
        Switch(
            checked = orientation == Prefs.ORIENTATION_AUTO,
            modifier = Modifier.semantics { contentDescription = "Rotate automatically" },
            onCheckedChange = { auto ->
                onOrientationChange(
                    if (auto) Prefs.ORIENTATION_AUTO else Prefs.ORIENTATION_CAMERA_DOWN
                )
            },
            colors = SwitchDefaults.colors(checkedTrackColor = Accent)
        )
    }
    if (orientation != Prefs.ORIENTATION_AUTO) {
        Spacer(Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(
                Prefs.ORIENTATION_CAMERA_UP to "Camera up",
                Prefs.ORIENTATION_CAMERA_DOWN to "Camera down",
            ).forEach { (value, label) ->
                FilterChip(
                    selected = orientation == value,
                    onClick = { onOrientationChange(value) },
                    label = { Text(label) },
                    colors = FilterChipDefaults.filterChipColors(
                        selectedContainerColor = Accent,
                        selectedLabelColor = Color.White
                    )
                )
            }
        }
        Text(
            "Camera down is the usual way to hold it for drawing.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
    Spacer(Modifier.height(20.dp))
}

@Composable
private fun StreamControls(bitrateMbps: Float, fpsChoice: Int, onBitrateChange: (Float) -> Unit, onFpsChange: (Int) -> Unit) {
    FrameRateControls(fpsChoice, onFpsChange)
    AdvancedSettings("Advanced video settings") { BitrateControls(bitrateMbps, onBitrateChange) }
    Spacer(Modifier.height(20.dp))
}

@Composable
private fun BitrateControls(bitrateMbps: Float, onBitrateChange: (Float) -> Unit) {
    val kilobits = (bitrateMbps * 1000).roundToInt()
    val label = java.math.BigDecimal.valueOf(kilobits.toLong(), 3).stripTrailingZeros().toPlainString()
    Text(
        "Bitrate: $label Mbps",
        fontSize = 14.sp,
        color = Color(0xFFB0B0C0)
    )
    Slider(
        value = bitrateMbps,
        onValueChange = onBitrateChange,
        valueRange = (Prefs.MIN_BITRATE_KBPS / 1000).toFloat()..
                (Prefs.MAX_BITRATE_KBPS / 1000).toFloat(),
        steps = (Prefs.MAX_BITRATE_KBPS - Prefs.MIN_BITRATE_KBPS) / 1000 - 1,
        colors = SliderDefaults.colors(thumbColor = Accent, activeTrackColor = Accent)
    )
    Text(
        "This setting depends on your computer's encoder. VAAPI constant quality (CQP) has no bitrate cap; " +
            "adjust Quality in Blent on your computer instead. Auto may select VAAPI.",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant
    )
    Spacer(Modifier.height(20.dp))
}

@Composable
private fun FrameRateControls(fpsChoice: Int, onFpsChange: (Int) -> Unit) {
    Text("Frame rate", fontSize = 14.sp, color = Color(0xFFB0B0C0))
    Spacer(Modifier.height(8.dp))
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        // Blent supports at most 90 FPS. The host also validates the
        // selected resolution/FPS against the generated EDID clock limit.
        listOf(30, 60, 90).forEach { f ->
            FilterChip(
                selected = fpsChoice == f,
                onClick = { onFpsChange(f) },
                label = { Text("$f fps") },
                colors = FilterChipDefaults.filterChipColors(
                    selectedContainerColor = Accent,
                    selectedLabelColor = Color.White
                )
            )
        }
    }
    Spacer(Modifier.height(20.dp))
}

@Composable
private fun SettingsSwitch(title: String, description: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.weight(1f)) {
            Text(title, fontSize = 14.sp, color = Color(0xFFB0B0C0))
            Text(description, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Switch(checked = checked, onCheckedChange = onChange,
            modifier = Modifier.semantics { contentDescription = title },
            colors = SwitchDefaults.colors(checkedTrackColor = Accent))
    }
}

@Composable
private fun UpdateNotice(updateAvailable: String?, onOpenUpdate: () -> Unit) {
    if (updateAvailable != null) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(10.dp))
                .background(Color(0x3350507A))
                .clickable { onOpenUpdate() }
                .padding(12.dp)
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text("Update available: $updateAvailable", fontSize = 14.sp, color = Color.White)
                Text(
                    "Tap to open the release page. Update the desktop side too — they ship together.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
        Spacer(Modifier.height(16.dp))
    }
}

@Composable
private fun ModeControl(penOnly: Boolean, onChange: (Boolean) -> Unit) {
    // Stream controls fold away when the tablet is only used for drawing.
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text("Graphics tablet", fontSize = 14.sp, color = Color(0xFFB0B0C0))
            Text(
                "Draw on the computer's own screen with the pen, " +
                    "instead of showing a second screen here",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        Spacer(Modifier.width(12.dp))
        Switch(
            checked = penOnly,
            modifier = Modifier.semantics { contentDescription = "Graphics tablet" },
            onCheckedChange = onChange,
            colors = SwitchDefaults.colors(checkedTrackColor = Accent)
        )
    }
    Spacer(Modifier.height(20.dp))
}
