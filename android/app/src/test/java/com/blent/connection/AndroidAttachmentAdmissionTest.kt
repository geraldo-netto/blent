package com.blent.connection

import android.content.BroadcastReceiver
import android.content.Intent
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.os.Looper
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.annotation.Resetter
import org.robolectric.shadows.ShadowUsbManager
import org.robolectric.util.ReflectionHelpers
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@Implements(UsbManager::class)
class AttachmentSnapshotShadow : ShadowUsbManager() {
    companion object {
        var entered: CountDownLatch? = null
        var release: CountDownLatch? = null
        @JvmStatic @Resetter fun resetGate() { entered = null; release = null }
    }
    @Implementation override fun getAccessoryList(): Array<UsbAccessory>? {
        entered?.countDown()
        release?.await()
        return super.getAccessoryList()
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AttachmentSnapshotShadow::class])
class AndroidAttachmentAdmissionTest {
    @Test fun t1405MissingNativeAccessoryAndManualCutoffRejectIntentAuthority() = runBlocking {
        NativeAttachmentFixture().use { native ->
            val app = RuntimeEnvironment.getApplication()
            val owner = AndroidAttachmentOwner.get(app)
            assertSame(owner, AndroidAttachmentOwner.get(app))
            native.absent()
            assertFalse(owner.take())
            native.reattach()
            assertTrue(owner.take())
            assertFalse(owner.take())
            native.reattach()
            owner.consume()
            assertFalse(owner.take())
            native.reattach()
            shadowOf(app.getSystemService(UsbManager::class.java)).setAttachedUsbAccessory(nativeAccessory())
            assertFalse("Unsupported native metadata cannot grant attachment", owner.take())
        }
    }

    private fun unregister(owner: AndroidAttachmentAdmission) {
        val app = RuntimeEnvironment.getApplication()
        val receiver = ReflectionHelpers.getField<BroadcastReceiver>(owner, "receiver")
        app.unregisterReceiver(receiver)
    }

    @Test fun t1405CancellationRetainsOriginalSnapshotAndRejectsStaleNativeResult() = runBlocking {
        NativeAttachmentFixture().use {
            val app = RuntimeEnvironment.getApplication()
            val owner = AndroidAttachmentAdmission(app)
            assertFalse(owner.take()) // Registration is required, not inferred from an Intent.
            owner.register()
            val entered = CountDownLatch(1)
            val release = CountDownLatch(1)
            AttachmentSnapshotShadow.entered = entered
            AttachmentSnapshotShadow.release = release
            val original = async(Dispatchers.Default) { owner.take() }
            try {
                assertTrue(entered.await(3, TimeUnit.SECONDS))
                original.cancel()
                assertFalse(original.isCompleted)
                assertFalse("One retained native snapshot", owner.take())
                owner.consume()
                release.countDown()
                original.join()
                assertFalse(owner.take())
                app.sendBroadcast(Intent(UsbManager.ACTION_USB_ACCESSORY_DETACHED))
                shadowOf(Looper.getMainLooper()).idle()
                assertTrue(owner.take())
            } finally {
                release.countDown(); original.cancelAndJoin(); unregister(owner)
                AttachmentSnapshotShadow.resetGate()
            }
        }
    }
    @Test fun t1521PresenceDoesNotConsumeAttachmentAndRequiresSupportedCurrentDevice() = runBlocking {
        NativeAttachmentFixture().use { native ->
            val app = RuntimeEnvironment.getApplication()
            val owner = AndroidAttachmentAdmission(app, Dispatchers.Unconfined)
            assertFalse(owner.present())
            owner.register()
            try {
                assertTrue(owner.present())
                assertTrue(owner.present())
                assertTrue(owner.take())
                assertTrue(owner.present())
                assertFalse(owner.take())
                native.absent()
                assertFalse(owner.present())
                native.reattach()
                assertTrue(owner.present())
                shadowOf(app.getSystemService(UsbManager::class.java)).setAttachedUsbAccessory(nativeAccessory())
                assertFalse(owner.present())
            } finally { unregister(owner) }
        }
    }

    @Test fun t1521PresenceSharesSnapshotOwnershipAndRejectsCutoffDuringNativeCall() = runBlocking {
        NativeAttachmentFixture().use {
            val owner = AndroidAttachmentAdmission(RuntimeEnvironment.getApplication())
            owner.register()
            val entered = CountDownLatch(1)
            val release = CountDownLatch(1)
            AttachmentSnapshotShadow.entered = entered
            AttachmentSnapshotShadow.release = release
            val original = async(Dispatchers.Default) { owner.present() }
            try {
                assertTrue(entered.await(3, TimeUnit.SECONDS))
                assertFalse(owner.present())
                assertFalse(owner.take())
                owner.consume()
                release.countDown()
                assertFalse(original.await())
                assertTrue(owner.present())
                assertFalse(owner.take())
            } finally {
                release.countDown(); original.cancelAndJoin(); unregister(owner)
                AttachmentSnapshotShadow.resetGate()
            }
        }
    }
}
