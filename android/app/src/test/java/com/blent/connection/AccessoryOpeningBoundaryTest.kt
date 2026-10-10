package com.blent.connection

import android.app.Application
import android.hardware.usb.UsbManager
import java.io.ByteArrayInputStream
import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AccessoryOpeningBoundaryTest {
    private val app: Application get() = RuntimeEnvironment.getApplication()
    private val manager: UsbManager get() = app.getSystemService(UsbManager::class.java)

    @Test fun t887FailedBridgeCloseCannotEstablishRetirement() {
        val device = nativeAccessory()
        val descriptor = accessoryFile()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { true }, opener = { descriptor })
        val opening = AndroidAccessoryOpening(inventory, accessorySelection(device), { true }, ports = { owned ->
            AndroidAccessoryPorts.open(owned, input = { copy -> object : ByteArrayInputStream(byteArrayOf()) {
                override fun close() { copy.close(); throw IOException("Native close failed") }
            } })
        })
        opening.open()
        assertThrows(IOException::class.java) { opening.close() }
        opening.close()
        assertFalse("T887: a failed native close does not prove retirement", opening.retired())
    }
    @Test fun t887FailedLatePortCleanupRetainsOwnership() {
        val device = nativeAccessory()
        val descriptor = accessoryFile()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { true }, opener = { descriptor })
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val opening = AndroidAccessoryOpening(inventory, accessorySelection(device), { true }, ports = { owned ->
            val ports = AndroidAccessoryPorts.open(owned, input = { copy -> object : ByteArrayInputStream(byteArrayOf()) {
                override fun close() { copy.close(); throw IOException("Late native close failed") }
            } })
            entered.countDown(); check(release.await(3, TimeUnit.SECONDS)); ports
        })
        val error = AtomicReference<Exception?>()
        val worker = Thread { try { opening.open() } catch (failure: Exception) { error.set(failure) } }
        worker.start(); assertTrue(entered.await(3, TimeUnit.SECONDS))
        opening.close(); assertFalse(opening.retired())
        release.countDown(); worker.join(3000)
        assertFalse(worker.isAlive)
        assertTrue(error.get() is IOException)
        assertFalse("T887: successful early close cannot hide failed late cleanup", opening.retired())
    }
    @Test fun t887FailedBridgeStartRetainsPumpsUntilActualCloseAndExit() {
        val device = nativeAccessory()
        val descriptor = accessoryFile()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { true }, opener = { descriptor })
        val admission = SocketBridgeAdmission(threads = { name, task ->
            if (name.endsWith("output")) object : Thread(task, name) {
                override fun start() { throw IOException("Native thread creation failed") }
            } else Thread(task, name)
        })
        val opening = AndroidAccessoryOpening(inventory, accessorySelection(device), { true }, admission)
        assertThrows(IOException::class.java) { opening.open() }
        assertFalse(opening.retired())
        opening.close()
        accessoryAwait(opening::retired)
    }
    @Test fun t887RevokedSelectionBeforeBridgeStartClosesUnstartedOwner() {
        val device = nativeAccessory()
        val descriptor = accessoryFile()
        val inventory = AndroidAccessoryInventory(manager, accessoryProfile, list = { arrayOf(device) }, permission = { true }, opener = { descriptor })
        var current = true
        val admission = SocketBridgeAdmission(sockets = { current = false; SocketBridgeSockets.open() })
        val opening = AndroidAccessoryOpening(inventory, accessorySelection(device), { current }, admission)
        assertThrows(IllegalStateException::class.java) { opening.open() }
        assertFalse(opening.retired())
        opening.close()
        assertTrue(opening.retired())
        val unopened = AndroidAccessoryOpening(inventory, accessorySelection(device), { true })
        unopened.close(); assertTrue(unopened.retired())
        assertThrows(IllegalStateException::class.java) { unopened.open() }
        assertTrue(unopened.retired())
    }
}
