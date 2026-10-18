package com.blent

import com.blent.activity.DisplayActivityUpdate
import com.blent.activity.DisplayActivityView

/** Captures only the pure source reader and exact immutable snapshot. No UI,
 * socket, callback or notification-flow owner is retained by a native request. */
internal data class ActivityPowerPermit(
    private val source: DisplayActivityView,
    private val snapshot: DisplayActivityUpdate,
) : DisplayPowerPermit {
    override fun current(): Boolean = source.current(snapshot)
}
