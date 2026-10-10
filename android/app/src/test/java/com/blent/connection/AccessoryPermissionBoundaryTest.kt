package com.blent.connection

import android.app.Application
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbManager
import android.os.Handler
import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
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
import org.robolectric.shadows.ShadowPendingIntent

@Implements(PendingIntent::class)
class AccessoryPendingShadow : ShadowPendingIntent() {
    companion object {
        var failCancel = false
        var beforeCancel: (() -> Unit)? = null
        @JvmStatic @Resetter fun resetFailure() { failCancel = false; beforeCancel = null }
    }
    @Implementation override fun cancel() {
        beforeCancel?.invoke()
        if (failCancel) throw IOException("Native pending cancellation failed")
        super.cancel()
    }
}
private class AccessoryRegistrationContext(
    app: Application, private val before: () -> Unit, private val after: () -> Unit = {},
) : ContextWrapper(app) {
    override fun getApplicationContext(): Context = this
    override fun registerReceiver(receiver: BroadcastReceiver?, filter: IntentFilter, permission: String?, scheduler: Handler?): Intent? {
        before(); return super.registerReceiver(receiver, filter, permission, scheduler).also { after() }
    }
    override fun registerReceiver(receiver: BroadcastReceiver?, filter: IntentFilter, permission: String?, scheduler: Handler?, flags: Int): Intent? {
        before(); return super.registerReceiver(receiver, filter, permission, scheduler, flags).also { after() }
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AccessoryUsbShadow::class, AccessoryPendingShadow::class])
class AccessoryPermissionBoundaryTest {
    private val app: Application get() = RuntimeEnvironment.getApplication()
    private val manager: UsbManager get() = app.getSystemService(UsbManager::class.java)

    @Test fun t887FailedCallbackCancellationRetainsPendingForNativeClose() {
        val device = nativeAccessory()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { true })
        val permission = AndroidAccessoryPermission(app, manager, inventory, { it() }) { _, _, _ -> }
        permission.start(); permission.request(accessorySelection(device))
        val pending = checkNotNull(AccessoryUsbShadow.requested)
        AccessoryPendingShadow.failCancel = true
        permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(pending).savedIntent)
        assertFalse(shadowOf(pending).isCanceled)
        assertNotNull(permission.failure())
        assertFalse(permission.retired())
        AccessoryPendingShadow.failCancel = false
        permission.close()
        assertTrue("T887: failed cancellation must retain its native PendingIntent until close", shadowOf(pending).isCanceled)
    }
    @Test fun t887ReplacementDuringBlockedCancelCannotCreateUnownedNativeToken() {
        val device = nativeAccessory()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { true })
        val work = ArrayList<() -> Unit>()
        val permission = AndroidAccessoryPermission(app, manager, inventory, work::add) { _, _, _ -> }
        val owner = AccessorySelectionOwner(accessoryProfile)
        permission.start(); permission.request(owner.select(inventory.snapshot()))
        val previous = checkNotNull(AccessoryUsbShadow.requested)
        val intent = shadowOf(previous).savedIntent
        permissionReceivers(app).single().broadcastReceiver.onReceive(app, intent)
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        AccessoryPendingShadow.beforeCancel = { entered.countDown(); check(release.await(3, TimeUnit.SECONDS)) }
        val worker = Thread { work.removeAt(0)() }
        worker.start(); assertTrue(entered.await(3, TimeUnit.SECONDS))
        val next = owner.select(inventory.snapshot())
        assertThrows(IllegalStateException::class.java) { permission.request(next) }
        val candidate = Intent(intent).setData(android.net.Uri.parse("blent-usb:${next.encode()}"))
        val orphan = PendingIntent.getBroadcast(app, 0, candidate, PendingIntent.FLAG_NO_CREATE or PendingIntent.FLAG_IMMUTABLE)
        release.countDown(); worker.join(3000)
        AccessoryPendingShadow.beforeCancel = null
        permission.close()
        try {
            assertFalse(worker.isAlive)
            assertNull("T887: replacement cannot create a native token while prior cancellation owns the slot", orphan)
            assertTrue(permission.retired())
        } finally { orphan?.cancel() }
    }
    @Test fun t887NativeRegistrationSideEffectThenFailureStillOwnsReceiver() {
        val device = nativeAccessory()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) })
        val context = AccessoryRegistrationContext(app, {}, { throw IOException("Registration returned an error after taking effect") })
        val permission = AndroidAccessoryPermission(context, manager, inventory, { it() }) { _, _, _ -> }
        assertThrows(IOException::class.java) { permission.start() }
        assertNotNull(permission.failure())
        assertFalse(permission.retired())
        assertEquals(1, permissionReceivers(app).size)
        permission.close()
        assertTrue(permissionReceivers(app).isEmpty())
        assertTrue(permission.retired())
    }
    @Test fun t887RegistrationReturningAfterCloseCannotLeaveReceiverRegistered() {
        val device = nativeAccessory()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) })
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val context = AccessoryRegistrationContext(app, { entered.countDown(); check(release.await(3, TimeUnit.SECONDS)) })
        val permission = AndroidAccessoryPermission(context, manager, inventory, { it() }) { _, _, _ -> }
        val startError = AtomicReference<Exception?>()
        val worker = Thread { try { permission.start() } catch (failure: Exception) { startError.set(failure) } }
        worker.start()
        assertTrue(entered.await(3, TimeUnit.SECONDS))
        val closeError = try { permission.close(); null } catch (failure: Exception) { failure }
        assertFalse("T887: close cannot retire a still-running native registration", permission.retired())
        release.countDown(); worker.join(3000)
        assertFalse(worker.isAlive)
        assertNull("T887: close before native registration must retain late cleanup without unregistering an absent receiver", closeError)
        assertTrue("T887: late native registration must be unregistered by its owner", permissionReceivers(app).isEmpty())
        assertTrue(permission.retired())
    }
}
