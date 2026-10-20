package com.blent

/** Read-only screen intent, independent of Android windows, media implementations
 * and native wake ownership. A demand never grants media access or retirement. */
internal fun interface ScreenWakeDemand {
    fun keepScreenOn(): Boolean
}

internal fun combinedScreenWake(display: ScreenWakeDemand, media: ScreenWakeDemand): ScreenWakeDemand =
    ScreenWakeDemand { display.keepScreenOn() || media.keepScreenOn() }
