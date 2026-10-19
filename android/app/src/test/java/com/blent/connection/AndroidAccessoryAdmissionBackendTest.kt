package com.blent.connection

import android.app.Application
import android.content.BroadcastReceiver
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbManager
import androidx.core.content.ContextCompat
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

private class AdmissionContext(app: Application) : ContextWrapper(app) {
    var before: () -> Unit = {}
    var after: () -> Unit = {}
    var failUnregister = false
    var detach: BroadcastReceiver? = null
    override fun getApplicationContext(): Context = this
    private fun before(receiver: BroadcastReceiver?, filter: IntentFilter) {
        if (filter.hasAction(UsbManager.ACTION_USB_ACCESSORY_DETACHED)) { detach = receiver; before() }
    }
    private fun after(filter: IntentFilter) { if (filter.hasAction(UsbManager.ACTION_USB_ACCESSORY_DETACHED)) after() }
    override fun registerReceiver(receiver: BroadcastReceiver?, filter: IntentFilter): Intent? {
        before(receiver, filter)
        // This legacy overload observes the protected system detach broadcast.
        return ContextCompat.registerReceiver(baseContext, receiver, filter,
            ContextCompat.RECEIVER_EXPORTED).also { after(filter) }
    }
    override fun registerReceiver(receiver: BroadcastReceiver?, filter: IntentFilter, flags: Int): Intent? {
        before(receiver, filter); return super.registerReceiver(receiver, filter, flags).also { after(filter) }
    }
    override fun unregisterReceiver(receiver: BroadcastReceiver) {
        if (receiver === detach && failUnregister) throw IOException("Detach unregister failed")
        super.unregisterReceiver(receiver)
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AccessoryUsbShadow::class])
class AndroidAccessoryAdmissionBackendTest {
    private val app: Application get() = RuntimeEnvironment.getApplication()
    private val manager: UsbManager get() = app.getSystemService(UsbManager::class.java)
    private fun detaches() = shadowOf(app).registeredReceivers.filter { it.intentFilter.hasAction(UsbManager.ACTION_USB_ACCESSORY_DETACHED) }
    private fun start(backend: AccessoryAdmissionBackend, detached: () -> Unit = {}) = backend.start({ it() }, { _, _, _ -> }, detached)
    private fun await(gate: CountDownLatch) { check(gate.await(3, TimeUnit.SECONDS)) }
    private fun join(thread: Thread) { thread.join(3000); assertFalse(thread.isAlive) }
    private fun launch(action: () -> Unit): Pair<Thread, AtomicReference<Exception?>> {
        val error = AtomicReference<Exception?>()
        return Thread { try { action() } catch (failure: Exception) { error.set(failure) } }.also { it.start() } to error
    }
    @Test fun t889ConstructorAndUnstartedCloseDoNoNativeLookup() {
        var queries = 0
        val backend = AndroidAccessoryAdmissionBackend({ queries++; app }, accessoryProfile)
        assertEquals(0, queries); assertFalse(backend.retired()); assertNull(backend.failure())
        backend.close(); backend.close(); assertTrue(backend.retired()); assertEquals(0, queries)
        assertThrows(IllegalStateException::class.java) { start(backend) }
        for (capacity in listOf(Int.MIN_VALUE, -1, 0, 1025, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { AndroidAccessoryAdmissionBackend({ app }, accessoryProfile, capacity) }
        }
    }
    @Test fun t889InventoryAndPermissionCallbackRecheckNativeGrant() {
        val device = nativeAccessory(); shadowOf(manager).setAttachedUsbAccessory(device)
        val work = ArrayList<() -> Unit>(); val reports = ArrayList<Boolean>()
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile)
        backend.start(work::add, { _, snapshot, permitted ->
            assertEquals(listOf(AndroidAccessoryInventory.describe(device)), snapshot); reports.add(permitted)
        }, {})
        try {
            assertEquals(listOf(AndroidAccessoryInventory.describe(device)), backend.snapshot())
            val selection = accessorySelection(device); assertFalse(backend.permitted(selection))
            backend.request(selection)
            val pending = checkNotNull(AccessoryUsbShadow.requested)
            permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(pending).savedIntent)
            assertTrue(reports.isEmpty()); shadowOf(manager).grantPermission(device)
            work.single()(); assertEquals(listOf(true), reports); assertTrue(backend.permitted(selection))
            assertNull(backend.failure()); assertFalse(backend.retired())
        } finally { backend.close() }
        assertTrue(backend.retired()); assertTrue(permissionReceivers(app).isEmpty()); assertTrue(detaches().isEmpty())
    }
    @Test fun t889DetachImmediatelyInvalidatesWithoutNativeQueriesAndCoalesces() {
        var queries = 0; var detached = 0; val device = nativeAccessory()
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile,
            inventoryFactory = { AndroidAccessoryInventory(it, accessoryProfile, list = { queries++; arrayOf(device) }) })
        start(backend) { detached++ }
        val receiver = detaches().single().broadcastReceiver
        receiver.onReceive(app, null); receiver.onReceive(app, Intent("wrong")); assertEquals(0, detached)
        receiver.onReceive(app, Intent(UsbManager.ACTION_USB_ACCESSORY_DETACHED))
        receiver.onReceive(app, Intent(UsbManager.ACTION_USB_ACCESSORY_DETACHED))
        assertEquals(1, detached); assertEquals(0, queries)
        assertThrows(IllegalStateException::class.java) { backend.snapshot() }
        backend.close(); receiver.onReceive(app, Intent(UsbManager.ACTION_USB_ACCESSORY_DETACHED))
        assertEquals(1, detached); assertTrue(backend.retired())
    }
    @Test fun t889DetachCallbackFailurePoisonsAdmissionButRetainsCleanup() {
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile)
        start(backend) { throw IOException("Owner callback failed") }
        detaches().single().broadcastReceiver.onReceive(app, Intent(UsbManager.ACTION_USB_ACCESSORY_DETACHED))
        assertTrue(backend.failure() is IOException); backend.close(); assertTrue(backend.retired())
    }
    @Test fun t889PermissionQueueFailureIsObservableWithoutReceiverNativeCleanup() {
        val device = nativeAccessory(); shadowOf(manager).setAttachedUsbAccessory(device)
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile)
        backend.start({ throw IOException("Full dispatch queue") }, { _, _, _ -> error("must not report") }, {})
        backend.request(accessorySelection(device)); val pending = checkNotNull(AccessoryUsbShadow.requested)
        permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(pending).savedIntent)
        assertTrue(backend.failure() is IOException); assertFalse(shadowOf(pending).isCanceled)
        backend.close(); assertTrue(shadowOf(pending).isCanceled); assertTrue(backend.retired())
    }
    @Test fun t889LateDetachRegistrationAfterCloseIsUnregisteredBeforeRetirement() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val context = AdmissionContext(app).apply { before = { entered.countDown(); await(release) } }
        val backend = AndroidAccessoryAdmissionBackend({ context }, accessoryProfile)
        val (worker, error) = launch { start(backend) }
        try { await(entered); backend.close(); assertFalse(backend.retired()) }
        finally { release.countDown(); join(worker) }
        assertTrue(error.get() is IllegalStateException); assertTrue(detaches().isEmpty()); assertTrue(backend.retired())
    }
    @Test fun t889RegistrationSideEffectThenFailureKeepsActualReceiverOwned() {
        val context = AdmissionContext(app).apply { after = { throw IOException("Registration failed after side effect") } }
        val backend = AndroidAccessoryAdmissionBackend({ context }, accessoryProfile)
        assertThrows(IOException::class.java) { start(backend) }
        assertEquals(1, detaches().size); assertNotNull(backend.failure()); assertFalse(backend.retired())
        backend.close(); assertTrue(detaches().isEmpty()); assertTrue(backend.retired())
    }
    @Test fun t889LatePermissionFactoryCannotEscapeClose() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile,
            permissionFactory = { context, manager, inventory, dispatch, report ->
                entered.countDown(); await(release); AndroidAccessoryPermission(context, manager, inventory, dispatch, report)
            })
        val (worker, error) = launch { start(backend) }
        try { await(entered); backend.close(); assertFalse(backend.retired()) }
        finally { release.countDown(); join(worker) }
        assertTrue(error.get() is IllegalStateException); assertTrue(detaches().isEmpty())
        assertTrue(permissionReceivers(app).isEmpty()); assertTrue(backend.retired())
    }
    @Test fun t889UnregisterFailureRetainsReceiverForCheckedRetry() {
        val context = AdmissionContext(app); val backend = AndroidAccessoryAdmissionBackend({ context }, accessoryProfile)
        start(backend); context.failUnregister = true
        assertThrows(IOException::class.java) { backend.close() }
        assertFalse(backend.retired()); assertEquals(1, detaches().size); assertTrue(backend.failure() is IOException)
        assertTrue(permissionReceivers(app).isEmpty())
        context.failUnregister = false; backend.close(); assertTrue(backend.retired()); assertTrue(detaches().isEmpty())
    }
    @Test fun t889BlockedNativeInventoryCannotBeDeclaredRetiredByClose() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile,
            inventoryFactory = { AndroidAccessoryInventory(it, accessoryProfile, list = { entered.countDown(); await(release); emptyArray() }) })
        start(backend); val (worker, error) = launch { backend.snapshot() }
        try { await(entered); backend.close(); assertFalse(backend.retired()) }
        finally { release.countDown(); join(worker) }
        assertNull(error.get()); assertTrue(backend.retired())
    }
    @Test fun t889InvalidInventoryAndRepeatedStartRejectWithoutReplacement() {
        val device = nativeAccessory(); var lists = 0
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile, capacity = 1,
            inventoryFactory = { AndroidAccessoryInventory(it, accessoryProfile, 1, list = { lists++; arrayOf(device, device) }) })
        start(backend); assertThrows(IllegalStateException::class.java) { start(backend) }
        assertThrows(IllegalArgumentException::class.java) { backend.snapshot() }
        assertTrue(backend.failure() is IllegalArgumentException)
        assertThrows(IllegalStateException::class.java) { backend.snapshot() }; assertEquals(1, lists)
        backend.close(); assertTrue(backend.retired())
    }
    @Test fun t889ThrowingNativeFactoriesRetainUncertainPermissionOwnership() {
        val context = AndroidAccessoryAdmissionBackend({ throw IOException("No context") }, accessoryProfile)
        assertThrows(IOException::class.java) { start(context) }; context.close(); assertTrue(context.retired())
        val permission = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile,
            permissionFactory = { _, _, _, _, _ -> throw IOException("Partial permission construction") })
        assertThrows(IOException::class.java) { start(permission) }; permission.close()
        assertFalse(permission.retired()); assertTrue(detaches().isEmpty())
    }
    @Test fun t889RealOwnerPermissionGrantThenDetachRetiresBothReceivers() {
        AccessoryUsbShadow.requested = null
        val device = nativeAccessory(); shadowOf(manager).setAttachedUsbAccessory(device)
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile)
        val owner = AccessoryAdmissionOwner(backend, accessoryProfile, clock = { 0L })
        try {
            accessoryAwait { AccessoryUsbShadow.requested != null }
            val pending = checkNotNull(AccessoryUsbShadow.requested)
            assertNull(owner.takeReady()); shadowOf(manager).grantPermission(device)
            permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(pending).savedIntent)
            var ready: AccessorySelection? = null
            accessoryAwait { ready = ready ?: owner.takeReady(); ready != null }
            val selection = checkNotNull(ready); assertTrue(owner.current(selection))
            detaches().single().broadcastReceiver.onReceive(app, Intent(UsbManager.ACTION_USB_ACCESSORY_DETACHED))
            assertFalse("Detach invalidation precedes all asynchronous cleanup", owner.current(selection))
            accessoryAwait(owner::retired)
            assertTrue(detaches().isEmpty()); assertTrue(permissionReceivers(app).isEmpty())
            assertTrue(shadowOf(pending).isCanceled); assertNull(owner.failure())
        } finally { owner.stop(); accessoryAwait(owner::retired) }
    }
    @Test fun t889RealOwnerTimerObservesPermissionCallbackDispatchFailure() {
        AccessoryUsbShadow.requested = null
        val device = nativeAccessory(); shadowOf(manager).setAttachedUsbAccessory(device)
        val backend = AndroidAccessoryAdmissionBackend({ app }, accessoryProfile,
            permissionFactory = { context, manager, inventory, _, report -> AndroidAccessoryPermission(
                context, manager, inventory, { throw IOException("Dispatch rejected") }, report)
            })
        val owner = AccessoryAdmissionOwner(backend, accessoryProfile, clock = { 0L })
        try {
            accessoryAwait { AccessoryUsbShadow.requested != null }
            val pending = checkNotNull(AccessoryUsbShadow.requested)
            permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(pending).savedIntent)
            assertTrue(backend.failure() is IOException); assertNull(owner.failure())
            owner.advance(); assertTrue(owner.failure() is IOException); assertNull(owner.takeReady())
            accessoryAwait(owner::retired)
            assertTrue(detaches().isEmpty()); assertTrue(permissionReceivers(app).isEmpty())
        } finally { owner.stop(); accessoryAwait(owner::retired) }
    }

}
