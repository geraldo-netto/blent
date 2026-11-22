package com.blent.connection

import java.util.concurrent.locks.LockSupport

/** Finite measurement pacing; caller bounds frame count and selects 0 or 60 fps. */
internal fun paceMeasurement(start: Long, frame: Int, fps: Int) {
    if (fps == 0) return
    val due = start + frame * (1_000_000_000L / fps)
    while (true) {
        val remaining = due - System.nanoTime()
        if (remaining <= 0) return
        LockSupport.parkNanos(remaining)
        check(!Thread.interrupted()) { "Measurement cancelled" }
    }
}
