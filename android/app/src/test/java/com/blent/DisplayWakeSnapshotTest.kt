package com.blent

import com.blent.activity.DisplayActivitySignal
import com.blent.activity.DisplayActivityState
import com.blent.activity.DisplayActivityUpdate
import org.junit.Assert.assertEquals
import org.junit.Test

class DisplayWakeSnapshotTest {
    private data class Row(
        val update: DisplayActivityUpdate?,
        val beforeAuthentication: DisplayWakeState,
        val beforeMetadata: DisplayWakeState,
        val ready: DisplayWakeState,
    )
    private fun row(state: DisplayActivityState?, known: Boolean, fallback: DisplayWakeState) = Row(
        state?.let { DisplayActivityUpdate(Any(), it, known) }, fallback, fallback, fallback,
    )
    private fun rows(): List<Row> {
        val paused = DisplayActivityUpdate(Any(), DisplayActivityState.Observed(DisplayActivitySignal(1, 0, false)))
        val active = DisplayActivityUpdate(Any(), DisplayActivityState.Observed(DisplayActivitySignal(1, 0, true)))
        return listOf(
            row(null, false, DisplayWakeState.WAITING),
            row(DisplayActivityState.Unavailable, false, DisplayWakeState.WAITING),
            row(DisplayActivityState.Unavailable, true, DisplayWakeState.UNAVAILABLE),
            row(DisplayActivityState.Unsupported, false, DisplayWakeState.UNSUPPORTED),
            Row(paused, DisplayWakeState.UNAVAILABLE, DisplayWakeState.UNAVAILABLE, DisplayWakeState.PAUSED),
            Row(active, DisplayWakeState.UNAVAILABLE, DisplayWakeState.UNAVAILABLE, DisplayWakeState.ACTIVE),
        )
    }
    private fun expected(row: Row, foreground: Boolean, authenticated: Boolean, metadata: Boolean): DisplayWakeState {
        if (!foreground) return DisplayWakeState.BACKGROUND
        if (!authenticated) return row.beforeAuthentication
        return if (metadata) row.ready else row.beforeMetadata
    }
    @Test fun t1061EveryOriginalSnapshotContextMatchesIndependentWakeTable() {
        for (row in rows()) {
            for (foreground in listOf(false, true)) {
                for (authenticated in listOf(false, true)) {
                    for (metadata in listOf(false, true)) {
                        val input = DisplayWakeSnapshot(foreground, authenticated, metadata, row.update)
                        assertEquals(expected(row, foreground, authenticated, metadata), input.wakeState())
                    }
                }
            }
        }
    }
}
