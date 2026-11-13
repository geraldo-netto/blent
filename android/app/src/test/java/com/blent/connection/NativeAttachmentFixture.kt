package com.blent.connection

import android.content.BroadcastReceiver
import android.content.Intent
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.os.Looper
import kotlinx.coroutines.Dispatchers
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.util.ReflectionHelpers

/** Owns this test's native snapshot, process adapter and receiver registration. */
internal class NativeAttachmentFixture : AutoCloseable {
    private val app = RuntimeEnvironment.getApplication()
    private val manager = app.getSystemService(UsbManager::class.java)
    private val previous = ReflectionHelpers.getStaticField<AndroidAttachmentAdmission?>(AndroidAttachmentOwner::class.java, "owner")
    private val previousDevice = manager.accessoryList?.singleOrNull()
    private val admission = AndroidAttachmentAdmission(app, Dispatchers.Unconfined)
    private val device = UsbAccessory::class.java.getDeclaredConstructor(*Array(6) { String::class.java })
        .newInstance("Blent", "Blent", "USB test", "1", "", "fixture")
    init {
        ReflectionHelpers.setStaticField(AndroidAttachmentOwner::class.java, "owner", admission)
        shadowOf(manager).setAttachedUsbAccessory(device)
    }
    fun absent() { shadowOf(manager).setAttachedUsbAccessory(null) }
    fun reattach() {
        absent()
        app.sendBroadcast(Intent(UsbManager.ACTION_USB_ACCESSORY_DETACHED))
        shadowOf(Looper.getMainLooper()).idle()
        shadowOf(manager).setAttachedUsbAccessory(device)
    }
    override fun close() {
        val receiver = ReflectionHelpers.getField<BroadcastReceiver>(admission, "receiver")
        if (shadowOf(app).registeredReceivers.any { it.broadcastReceiver === receiver }) app.unregisterReceiver(receiver)
        shadowOf(manager).setAttachedUsbAccessory(previousDevice)
        ReflectionHelpers.setStaticField(AndroidAttachmentOwner::class.java, "owner", previous)
    }
}
