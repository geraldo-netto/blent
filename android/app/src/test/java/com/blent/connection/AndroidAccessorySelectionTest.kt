package com.blent.connection

import android.app.Application
import android.app.PendingIntent
import android.content.Intent
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.os.Parcel
import android.os.ParcelFileDescriptor
import java.io.File
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
import org.robolectric.shadows.ShadowUsbManager

@Implements(UsbManager::class)
class AccessoryUsbShadow : ShadowUsbManager() {
    companion object {
        var requested: PendingIntent? = null
        var failure: Exception? = null
        @JvmStatic @Resetter fun resetRequests() { requested = null; failure = null }
    }
    @Implementation fun requestPermission(accessory: UsbAccessory, pending: PendingIntent) {
        requested = pending
        failure?.let { throw it }
    }
}

internal fun nativeAccessory(serial: String = "fixture"): UsbAccessory =
    UsbAccessory::class.java.getDeclaredConstructor(*Array(6) { String::class.java })
        .newInstance("Blent", "Fixture", "USB test", "1", "", serial)
internal fun permissionReceivers(app: Application) = shadowOf(app).registeredReceivers.filter {
    it.intentFilter.actionsIterator()?.asSequence()?.any { action -> action.startsWith("${app.packageName}.USB_PERMISSION.") } == true
}
internal val accessoryProfile = AccessoryProfile("Blent", "Fixture", "1")
internal fun accessorySelection(device: UsbAccessory): AccessorySelection =
    AccessorySelectionOwner(accessoryProfile).select(listOf(AndroidAccessoryInventory.describe(device)))
internal fun accessoryFile(): ParcelFileDescriptor {
    val file = File.createTempFile("blent-usb-open", ".fixture")
    return try { ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_WRITE) } finally { file.delete() }
}
internal fun accessoryAwait(condition: () -> Boolean) {
    val end = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
    while (!condition() && System.nanoTime() < end) Thread.sleep(1)
    assertTrue(condition())
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AccessoryUsbShadow::class])
class AndroidAccessorySelectionTest {
    private val app: Application get() = RuntimeEnvironment.getApplication()
    private val manager: UsbManager get() = app.getSystemService(UsbManager::class.java)

    @Test @Config(sdk = [34]) fun t887SelectionDoesNotReadPermissionGatedNativeSerial() {
        val type = Class.forName("android.hardware.usb.IUsbSerialReader")
        var reads = 0
        val reader = java.lang.reflect.Proxy.newProxyInstance(type.classLoader, arrayOf(type)) { _, _, _ ->
            reads++; throw SecurityException("USB permission required to read serial")
        }
        val constructor = UsbAccessory::class.java.getDeclaredConstructor(
            String::class.java, String::class.java, String::class.java,
            String::class.java, String::class.java, type,
        )
        val device = constructor.newInstance("Blent", "Fixture", "USB test", "1", "", reader)
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { false })
        val selected = AccessorySelectionOwner(accessoryProfile).select(inventory.snapshot())
        assertFalse(inventory.permitted(selected))
        assertEquals("T887: selecting an unapproved accessory must never query its permission-gated serial", 0, reads)
    }
    @Test fun t887NativeSnapshotsRequireExactUniqueProfileAndRealPermission() {
        val device = nativeAccessory()
        val shadow = shadowOf(manager)
        shadow.setAttachedUsbAccessory(device)
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile)
        val selected = accessorySelection(device)
        assertEquals(listOf(AndroidAccessoryInventory.describe(device)), inventory.snapshot())
        assertEquals(device, inventory.resolve(selected))
        assertFalse(inventory.permitted(selected))
        assertThrows(IllegalStateException::class.java) { inventory.open(selected) }
        shadow.grantPermission(device)
        assertTrue(inventory.permitted(selected))
        inventory.open(selected).close()
        shadow.revokePermission(device, app.packageName)
        assertFalse(inventory.permitted(selected))
    }
    @Test fun t887NativeDuplicatesDetachCapacityAndOpenFailureReject() {
        val device = nativeAccessory()
        val selected = accessorySelection(device)
        var devices: Array<UsbAccessory>? = arrayOf(device, device)
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { devices }, permission = { true }, opener = { null })
        assertThrows(IllegalStateException::class.java) { inventory.resolve(selected) }
        devices = null
        assertEquals(emptyList<AccessoryDescription>(), inventory.snapshot())
        assertThrows(IllegalStateException::class.java) { inventory.resolve(selected) }
        devices = arrayOf(device)
        assertThrows(IllegalStateException::class.java) { inventory.open(selected) }
        val bounded = AndroidAccessoryInventory(manager, accessoryProfile, capacity = 1, list = { arrayOf(device, device) })
        assertThrows(IllegalArgumentException::class.java) { bounded.snapshot() }
    }
    @Test fun t887ForgedGrantAndStalePermissionCallbackNeverAdmit() {
        val device = nativeAccessory()
        shadowOf(manager).setAttachedUsbAccessory(device)
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile)
        val owner = AccessorySelectionOwner(accessoryProfile)
        val work = ArrayList<() -> Unit>()
        val answers = ArrayList<Boolean>()
        val permission = AndroidAccessoryPermission(app, manager, inventory, work::add) { ticket, snapshot, permitted ->
            answers.add(owner.confirm(ticket, snapshot, permitted) != null)
        }
        permission.start()
        val selected = owner.select(inventory.snapshot())
        permission.request(selected)
        val first = checkNotNull(AccessoryUsbShadow.requested)
        assertTrue(shadowOf(first).isImmutable)
        val wrapper = permissionReceivers(app).single()
        assertEquals("${app.packageName}.permission.USB_CALLBACK", wrapper.broadcastPermission)
        val original = shadowOf(first).savedIntent
        wrapper.broadcastReceiver.onReceive(app, Intent(original).setAction("wrong"))
        wrapper.broadcastReceiver.onReceive(app, Intent(original).setData(null))
        wrapper.broadcastReceiver.onReceive(app, null)
        assertTrue(work.isEmpty())
        wrapper.broadcastReceiver.onReceive(app, Intent(original).putExtra(UsbManager.EXTRA_PERMISSION_GRANTED, true))
        assertEquals(1, work.size)
        work.removeAt(0)()
        assertEquals(listOf(false), answers)
        val next = owner.select(inventory.snapshot())
        permission.request(next)
        wrapper.broadcastReceiver.onReceive(app, original)
        assertTrue(work.isEmpty())
        shadowOf(manager).grantPermission(device)
        wrapper.broadcastReceiver.onReceive(app, shadowOf(checkNotNull(AccessoryUsbShadow.requested)).savedIntent)
        work.removeAt(0)()
        assertEquals(listOf(false, true), answers)
        permission.close(); permission.close()
        assertTrue(permissionReceivers(app).isEmpty())
    }
    @Test fun t887PermissionReplacementDetachQueueFailureAndCloseRejectCallbacks() {
        val device = nativeAccessory()
        var devices: Array<UsbAccessory>? = arrayOf(device)
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { devices }, permission = { true })
        val answers = ArrayList<Boolean>()
        val work = ArrayList<() -> Unit>()
        val permission = AndroidAccessoryPermission(app, manager, inventory, work::add) { _, _, allowed -> answers.add(allowed) }
        assertThrows(IllegalStateException::class.java) { permission.request(accessorySelection(device)) }
        permission.start()
        assertThrows(IllegalStateException::class.java) { permission.start() }
        val owner = AccessorySelectionOwner(accessoryProfile)
        permission.request(owner.select(inventory.snapshot()))
        val old = checkNotNull(AccessoryUsbShadow.requested)
        permission.request(owner.select(inventory.snapshot()))
        assertTrue(shadowOf(old).isCanceled)
        val pending = checkNotNull(AccessoryUsbShadow.requested)
        val receiver = permissionReceivers(app).single().broadcastReceiver
        receiver.onReceive(app, shadowOf(pending).savedIntent)
        devices = null
        work.removeAt(0)()
        assertEquals(listOf(false), answers)
        devices = arrayOf(device)
        permission.request(owner.select(inventory.snapshot()))
        receiver.onReceive(app, shadowOf(checkNotNull(AccessoryUsbShadow.requested)).savedIntent)
        permission.close()
        work.removeAt(0)()
        receiver.onReceive(app, shadowOf(pending).savedIntent)
        assertEquals(listOf(false), answers)
        assertThrows(IllegalStateException::class.java) { permission.request(owner.select(inventory.snapshot())) }
        val rejected = AndroidAccessoryPermission(app, manager, inventory, { throw IOException("Queue full") }) { _, _, allowed -> answers.add(allowed) }
        rejected.start(); rejected.request(owner.select(inventory.snapshot()))
        permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(checkNotNull(AccessoryUsbShadow.requested)).savedIntent)
        assertEquals("T887: failed enqueue only changes local failure state", listOf(false), answers)
        assertNotNull(rejected.failure())
        assertFalse(rejected.retired())
        rejected.close(); assertTrue(rejected.retired())
    }
    @Test fun t887SamePermissionTicketCannotReplaceQueuedRequest() {
        val device = nativeAccessory()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) })
        val work = ArrayList<() -> Unit>()
        val permission = AndroidAccessoryPermission(app, manager, inventory, work::add) { _, _, _ -> }
        permission.start()
        val selected = accessorySelection(device)
        permission.request(selected)
        val original = checkNotNull(AccessoryUsbShadow.requested)
        permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(original).savedIntent)
        assertThrows(IllegalStateException::class.java) { permission.request(selected) }
        assertFalse(shadowOf(original).isCanceled)
        permission.close()
        work.removeAt(0)()
    }
    @Test fun t887DispatchRejectionRetainsNativeCleanupForCloser() {
        val device = nativeAccessory()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) })
        val permission = AndroidAccessoryPermission(app, manager, inventory, { throw IOException("Queue full") }) { _, _, _ -> }
        permission.start(); permission.request(accessorySelection(device))
        val pending = checkNotNull(AccessoryUsbShadow.requested)
        permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(pending).savedIntent)
        assertFalse("T887: receiver must only revoke local admission; native cancel belongs to closer", shadowOf(pending).isCanceled)
        assertNotNull(permission.failure())
        permission.close()
        assertTrue(shadowOf(pending).isCanceled)
        assertTrue(permission.retired())
    }
    @Test fun t887NativeRequestFailureCancelsOwnedPendingIntent() {
        val device = nativeAccessory()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) })
        val permission = AndroidAccessoryPermission(app, manager, inventory, { it() }) { _, _, _ -> fail("No permission result") }
        permission.start()
        try {
            AccessoryUsbShadow.failure = IOException("Native request failed")
            assertThrows(IOException::class.java) { permission.request(accessorySelection(device)) }
            assertTrue(shadowOf(checkNotNull(AccessoryUsbShadow.requested)).isCanceled)
            permission.close()
            val unused = AndroidAccessoryPermission(app, manager, inventory, { it() }) { _, _, _ -> fail() }
            unused.close()
            assertThrows(IllegalStateException::class.java) { unused.start() }
        } finally {
            AccessoryUsbShadow.failure = null
            permission.close()
        }
    }
    @Test fun t887OpeningOwnsDescriptorBridgeAndRejectsRepeatedAdmission() {
        val device = nativeAccessory()
        val descriptor = accessoryFile()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { true }, opener = { descriptor })
        val opening = AndroidAccessoryOpening(inventory, accessorySelection(device), { true })
        assertFalse(opening.retired())
        assertTrue(opening.open().isConnected)
        assertThrows(IllegalStateException::class.java) { opening.open() }
        opening.close(); opening.close()
        accessoryAwait(opening::retired)
        assertThrows(IllegalStateException::class.java) { descriptor.fd }
    }
    @Test fun t887DetachAfterNativeOpenStillOwnsAndClosesDescriptor() {
        val device = nativeAccessory()
        var attached = true
        val descriptor = accessoryFile()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { if (attached) arrayOf(device) else null }, permission = { true }, opener = { attached = false; descriptor })
        val opening = AndroidAccessoryOpening(inventory, accessorySelection(device), { true })
        assertThrows(IllegalStateException::class.java) { opening.open() }
        assertFalse(opening.retired())
        opening.close()
        assertTrue(opening.retired())
        assertThrows(IllegalStateException::class.java) { descriptor.fd }
    }
    @Test fun t887LateDescriptorAfterStopClosesBeforeRetirement() {
        val device = nativeAccessory()
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val descriptor = accessoryFile()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { true }, opener = {
            entered.countDown(); check(release.await(3, TimeUnit.SECONDS)); descriptor
        })
        val opening = AndroidAccessoryOpening(inventory, accessorySelection(device), { true })
        val error = AtomicReference<Exception?>()
        val worker = Thread { try { opening.open() } catch (failure: Exception) { error.set(failure) } }
        worker.start()
        assertTrue(entered.await(3, TimeUnit.SECONDS))
        opening.close()
        assertFalse(opening.retired())
        release.countDown(); worker.join(3000)
        assertFalse(worker.isAlive); assertNotNull(error.get()); assertTrue(opening.retired())
        assertThrows(IllegalStateException::class.java) { descriptor.fd }
    }
    @Test fun t887StaleSelectionDeniedPermissionAndPortFailureNeverAdmit() {
        val device = nativeAccessory()
        val selected = accessorySelection(device)
        var permitted = false
        val descriptor = accessoryFile()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { permitted }, opener = { descriptor })
        val denied = AndroidAccessoryOpening(inventory, selected, { true })
        assertThrows(IllegalStateException::class.java) { denied.open() }
        denied.close(); assertTrue(denied.retired())
        permitted = true
        val stale = AndroidAccessoryOpening(inventory, selected, { false })
        assertThrows(IllegalStateException::class.java) { stale.open() }
        stale.close(); assertTrue(stale.retired())
        val broken = AndroidAccessoryOpening(inventory, selected, { true }, ports = { throw IOException("Ports failed") })
        assertThrows(IOException::class.java) { broken.open() }
        broken.close(); assertTrue(broken.retired())
        assertThrows(IllegalStateException::class.java) { descriptor.fd }
    }
}
