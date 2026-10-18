package com.blent

import android.content.Context
import android.net.wifi.WifiManager
import android.os.Build
import android.os.PowerManager

/** Backend-local API boundary. Probe is diagnostic unless combined with a
 * successful original release call; unknown acquisition requires process fence. */
internal interface PlatformWakeLock {
    val held: Boolean
    fun acquire()
    fun release()
}

internal class AndroidWakeHandle(private val lock: PlatformWakeLock) : WakeHandle {
    private var attempted = false
    private var confirmed = false
    override fun acquire() {
        check(!attempted)
        attempted = true
        lock.acquire()
        confirmed = true
    }
    override fun retire() {
        if (!attempted) return
        check(confirmed || lock.held) { "Original wake acquisition remains unknown; power process fence required" }
        confirmed = false
        lock.release()
        attempted = false
    }
}

internal object AndroidWakeHandles {
    fun cpu(context: Context): WakeHandle {
        val manager = context.applicationContext.getSystemService(Context.POWER_SERVICE) as PowerManager
        val lock = manager.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "Blent::StreamingWakeLock")
        lock.setReferenceCounted(false)
        return AndroidWakeHandle(object : PlatformWakeLock {
            override val held get() = lock.isHeld
            override fun acquire() { lock.acquire(CPU_WAKE_MILLIS) }
            override fun release() { lock.release() }
        })
    }
    fun wifi(context: Context): WakeHandle {
        val manager = context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
        val mode = if (Build.VERSION.SDK_INT >= 29) WifiManager.WIFI_MODE_FULL_LOW_LATENCY
            else @Suppress("DEPRECATION") WifiManager.WIFI_MODE_FULL_HIGH_PERF
        val lock = manager.createWifiLock(mode, "Blent::StreamingWifiLock")
        lock.setReferenceCounted(false)
        return AndroidWakeHandle(object : PlatformWakeLock {
            override val held get() = lock.isHeld
            override fun acquire() { lock.acquire() }
            override fun release() { lock.release() }
        })
    }
}
