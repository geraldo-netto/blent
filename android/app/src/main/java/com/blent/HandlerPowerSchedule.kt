package com.blent

import android.os.Handler
import android.os.Looper
import android.os.SystemClock

/** Native queue belongs to the application UI loop, never to a released Window. */
internal class HandlerPowerSchedule : PowerSchedule {
    private val handler = Handler(Looper.getMainLooper())
    override fun now() = SystemClock.uptimeMillis()
    override fun post(delayMillis: Long, action: Runnable): Boolean = handler.postDelayed(action, delayMillis)
    override fun cancel(action: Runnable) { handler.removeCallbacks(action) }
}
