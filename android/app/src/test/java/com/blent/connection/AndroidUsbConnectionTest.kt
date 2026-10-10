package com.blent.connection

import android.app.Application
import android.content.Intent
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AccessoryUsbShadow::class])
class AndroidUsbConnectionTest {
    private val app: Application get() = RuntimeEnvironment.getApplication()
    private val manager: UsbManager get() = app.getSystemService(UsbManager::class.java)
    private fun device(): UsbAccessory = UsbAccessory::class.java.getDeclaredConstructor(*Array(6) { String::class.java })
        .newInstance("Blent", "Blent", "Blent USB connection", "1", "", "fixture")
    private suspend fun until(condition: () -> Boolean) = withTimeout(3000) { while (!condition()) delay(1) }
    private val spec = ConnectionWorkerSpec("unselected", nativeTimeoutMs = 1000,
        approvalTimeoutMs = 3000, io = ConnectionRunLimits(1000, 1000), timerMs = 1)
    @Test fun t854ProductionEntryDoesNotQueryContextBeforeExecution() = runBlocking {
        var queries = 0
        val worker = androidUsbConnection({ queries++; app }, spec, {})
        worker.cancel(); worker.retire(); assertEquals(0, queries)
    }
    @Test fun t854ProductionEntryWaitsForRealPermissionAndRetiresActualBinding() = runBlocking {
        AccessoryUsbShadow.requested = null
        val device = device(); shadowOf(manager).setAttachedUsbAccessory(device)
        val endpoint = ConnectionWorkerEndpoint(); val context = ConnectionWorkerContext(endpoint)
        val worker = androidUsbConnection({ context }, spec, {}, { System.nanoTime() / 1000000 })
        val execution = async(Dispatchers.Default) { worker.execute() }
        try {
            until { AccessoryUsbShadow.requested != null }
            assertEquals(0, context.binds)
            shadowOf(manager).grantPermission(device)
            val pending = checkNotNull(AccessoryUsbShadow.requested)
            permissionReceivers(app).single().broadcastReceiver.onReceive(app, shadowOf(pending).savedIntent)
            withTimeout(3000) { execution.await(); worker.retire() }
            assertEquals(1, context.binds); assertEquals(1, context.unbinds); assertFalse(endpoint.alive)
            assertTrue(permissionReceivers(app).isEmpty())
        } finally { worker.cancel(); endpoint.die(); worker.retire() }
    }
    @Test fun t854DetachStopsBoundWorkerAndDiscardsStalePairing() = runBlocking {
        val device = device(); shadowOf(manager).setAttachedUsbAccessory(device); shadowOf(manager).grantPermission(device)
        val endpoint = ConnectionWorkerEndpoint().apply { start = {} }
        val context = ConnectionWorkerContext(endpoint)
        val events = java.util.Collections.synchronizedList(ArrayList<ConnectionWorkerEvent>())
        val worker = androidUsbConnection({ context }, spec, events::add, { System.nanoTime() / 1000000 })
        val execution = async(Dispatchers.Default) { runCatching { worker.execute() } }
        try {
            until { endpoint.callback != null }
            val receiver = shadowOf(app).registeredReceivers.single { it.intentFilter.hasAction(UsbManager.ACTION_USB_ACCESSORY_DETACHED) }
            receiver.broadcastReceiver.onReceive(app, Intent(UsbManager.ACTION_USB_ACCESSORY_DETACHED))
            val peer = ConnectionPeer.parse("ab".repeat(32))
            endpoint.emit(ConnectionWorkerEvent.Pairing(ConnectionTicket(1, peer), peer))
            until { endpoint.codes.contains(ConnectionWorkerIpc.STOP) }
            endpoint.emit(ConnectionWorkerEvent.Ended(null))
            withTimeout(3000) { execution.await(); worker.retire() }
            assertTrue(events.none { it is ConnectionWorkerEvent.Pairing })
            assertFalse(endpoint.alive); assertEquals(1, context.unbinds)
        } finally { worker.cancel(); endpoint.die(); worker.retire() }
    }
}
