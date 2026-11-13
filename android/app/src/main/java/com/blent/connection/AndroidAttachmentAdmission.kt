package com.blent.connection

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbManager
import android.os.Build
import kotlinx.coroutines.*
import kotlinx.coroutines.sync.Mutex

/** One application-owned receiver and at most one native snapshot. The receiver
 * intentionally lives for the process, including Activity absence and user cutoff.
 * NonCancellable retains the snapshot slot until the original native call returns;
 * cancellation cannot admit a parallel snapshot or publish a configured start.
 * Missing detach evidence fails closed; manual Connect remains available. */
internal class AndroidAttachmentAdmission(
    context: Context,
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
) : AttachmentAdmission {
    private val app = context.applicationContext
    private val epoch = AttachmentEpoch()
    private val snapshot = Mutex()
    private var registered = false
    private var registrationAttempted = false
    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == UsbManager.ACTION_USB_ACCESSORY_DETACHED) epoch.detached()
        }
    }

    /** Called once by the process owner before publishing this adapter. On an
     * uncertain registration failure, retain the owner and refuse admission. */
    fun register() {
        if (registrationAttempted) return
        registrationAttempted = true
        val filter = IntentFilter(UsbManager.ACTION_USB_ACCESSORY_DETACHED)
        try {
            if (Build.VERSION.SDK_INT >= 33) app.registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED)
            else app.registerReceiver(receiver, filter)
            registered = true
        } catch (_: Exception) { /* Retained, fail closed. */ }
    }

    override fun consume() = epoch.consume()

    override suspend fun take(): Boolean {
        currentCoroutineContext().ensureActive()
        if (!registered) return false
        val candidate = epoch.candidate() ?: return false
        val original = Any()
        if (!snapshot.tryLock(original)) return false
        try {
            return withContext(NonCancellable + dispatcher) {
                supportedAccessory() && epoch.claim(candidate)
            }
        } finally { snapshot.unlock(original) }
    }

    private fun supportedAccessory(): Boolean = try {
        val manager = checkNotNull(app.getSystemService(UsbManager::class.java))
        val profile = AndroidConnectionFactory.profile
        val devices = AndroidAccessoryInventory(manager, profile).snapshot()
        AccessoryResolver(profile).choose(devices)
        true
    } catch (_: Exception) { false }
}

internal object AndroidAttachmentOwner {
    private var owner: AndroidAttachmentAdmission? = null
    @Synchronized fun get(context: Context): AttachmentAdmission {
        val retained = owner ?: AndroidAttachmentAdmission(context).also { owner = it }
        retained.register()
        return retained
    }
}
