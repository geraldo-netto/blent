package com.blent

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.util.Log

class StreamingService internal constructor(
    private val suppliedRuntime: StreamingPowerRuntime? = null,
    private val schedule: PowerSchedule = HandlerPowerSchedule(),
) : Service() {
    companion object {
        const val CHANNEL_ID = "blent_streaming"
        const val NOTIFICATION_ID = 1
        private const val TAG = "BlentService"
    }

    private lateinit var runtime: StreamingPowerRuntime
    private var attached = false
    private var destroyed = false
    private var pending: Posted? = null
    private val port = object : PowerServicePort {
        override fun post(notice: DisplayPowerNotice) = postNotice(notice)
        override fun stop() { stopSelf() }
    }

    private class Posted(
        private val runtime: StreamingPowerRuntime,
        val notice: DisplayPowerNotice,
        private val deliver: (Posted) -> Unit,
    ) {
        private val live = java.util.concurrent.atomic.AtomicBoolean(true)
        private val done = java.util.concurrent.atomic.AtomicBoolean()
        val action = Runnable {
            if (done.compareAndSet(false, true)) {
                if (live.get()) deliver(this) else runtime.canceled(notice)
            }
        }
        fun refused() { live.set(false); done.set(true) }
        fun cancel(schedule: PowerSchedule) {
            live.set(false)
            schedule.cancel(action)
            if (done.compareAndSet(false, true)) runtime.canceled(notice)
        }
    }

    override fun onCreate() {
        super.onCreate()
        runtime = suppliedRuntime ?: StreamingPowerRuntime.forApp(this)
        createNotificationChannel()
        attached = promote() && runtime.attach(port)
        if (!attached) stopSelf()
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val envelope = PowerServiceNotice.read(intent)
        if (envelope?.runtime !== runtime) {
            if (!runtime.hasOwner) stopSelf()
            return START_NOT_STICKY
        }
        deliver(envelope, envelope.matches(requireNotNull(intent)))
        return START_NOT_STICKY
    }
    private fun deliver(envelope: PowerServiceNotice, matches: Boolean) {
        if (!runtime.pending(envelope.notice)) {
            if (!runtime.hasOwner) stopSelf()
            return
        }
        if (destroyed) { runtime.canceled(envelope.notice); return }
        if (!promote()) {
            runtime.detach(port); attached = false
            runtime.rejectedReceipt(envelope.notice, IllegalStateException("Foreground promotion rejected"))
            stopSelf()
            return
        }
        if (!attached) attached = runtime.attach(port)
        runtime.receive(port, envelope, matches && attached)
    }
    private fun postNotice(notice: DisplayPowerNotice): Boolean {
        if (!attached || destroyed) return false
        check(pending == null) { "Original power callback still admitted" }
        val original = Posted(runtime, notice) {
            if (pending === it) pending = null
            deliver(PowerServiceNotice(runtime, it.notice), true)
        }
        pending = original
        if (schedule.post(0, original.action)) return true
        if (pending === original) pending = null
        original.refused()
        return false
    }
    private fun promote(): Boolean = try {
        startForeground(NOTIFICATION_ID, notification())
        true
    } catch (error: Exception) {
        Log.w(TAG, "Foreground promotion rejected", error)
        false
    }

    private fun notification(): Notification {
        val pending = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        return Notification.Builder(this, CHANNEL_ID)
            .setContentTitle("Blent Active").setContentText("Streaming display to this device")
            .setSmallIcon(android.R.drawable.ic_menu_view).setContentIntent(pending).setOngoing(true).build()
    }

    override fun onDestroy() {
        destroyed = true
        val original = pending
        pending = null
        try {
            runtime.detach(port); attached = false
            try { original?.cancel(schedule) }
            catch (failure: Exception) { Log.w(TAG, "Original queued power callback remains owned", failure) }
        } finally { super.onDestroy() }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Blent Streaming",
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = "Keeps Blent alive while streaming"
            setShowBadge(false)
        }
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(channel)
    }
}
