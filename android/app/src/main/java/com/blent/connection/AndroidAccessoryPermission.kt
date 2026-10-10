package com.blent.connection

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbManager
import android.net.Uri
import android.os.Build
import java.io.Closeable
import java.util.UUID

/** Receiver work is limited to a local gate and one queued hint. Native methods
 * run on supervised workers; a separate closer may race them. Keep this owner
 * after errors until retired proves native operations and cleanup really ended. */
internal class AndroidAccessoryPermission(
    context: Context,
    private val manager: UsbManager,
    private val inventory: AndroidAccessoryInventory,
    private val dispatch: (() -> Unit) -> Unit,
    private val report: (AccessorySelection, List<AccessoryDescription>, Boolean) -> Unit,
) : Closeable, ConnectionSetupLifetime {
    private class Pending(val selection: AccessorySelection, val intent: PendingIntent) { var queued = false }
    private val app = context.applicationContext
    private val gate = Any()
    private val action = "${app.packageName}.USB_PERMISSION.${UUID.randomUUID()}"
    private var started = false
    private var registering = false
    private var registered = false
    private var closed = false
    private var requestBusy = false
    private var closeBusy = false
    private var cancelling = false
    private var unregistering = false
    private var operations = 0
    private var epoch = 0L
    private var error: Exception? = null
    private var pending: Pending? = null
    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val owned = claimHint(intent) ?: return
            try { dispatch { complete(owned) } } catch (failure: Exception) { failed(failure) }
        }
    }

    fun failure(): Exception? = synchronized(gate) { error }
    override fun retired(): Boolean = synchronized(gate) {
        closed && operations == 0 && !registered && pending == null && !registering
    }
    private fun failed(failure: Exception) { synchronized(gate) { if (error == null) error = failure } }
    private fun live() { check(!closed && error == null) { "Permission owner is closed or failed" } }
    private fun address(selection: AccessorySelection): Uri = Uri.parse("blent-usb:${selection.encode()}")

    private fun claimHint(intent: Intent?): Pending? = synchronized(gate) {
        if (closed || error != null || intent?.action != action) return null
        val owned = pending ?: return null
        if (owned.queued || owned.selection.epoch != epoch || intent.data != address(owned.selection)) return null
        owned.queued = true
        owned
    }

    fun start() {
        synchronized(gate) { live(); check(!started); started = true; registering = true; operations++ }
        try {
            register()
            synchronized(gate) { registered = true }
        } catch (failure: Exception) {
            // Registration may have taken effect before its caller saw failure.
            synchronized(gate) { registered = true }
            failed(failure); throw failure
        } finally {
            synchronized(gate) { registering = false }
            try { if (synchronized(gate) { closed }) cleanup() }
            finally { synchronized(gate) { operations-- } }
        }
    }
    private fun register() {
        val filter = IntentFilter(action)
        val permission = "${app.packageName}.permission.USB_CALLBACK"
        if (Build.VERSION.SDK_INT >= 33) app.registerReceiver(receiver, filter, permission, null, Context.RECEIVER_NOT_EXPORTED)
        else app.registerReceiver(receiver, filter, permission, null)
    }

    private fun beginRequest(selection: AccessorySelection) = synchronized(gate) {
        live(); check(registered && !registering && !requestBusy)
        check(selection.epoch > epoch) { "Permission request epoch must increase" }
        epoch = selection.epoch; requestBusy = true; operations++
    }
    fun request(selection: AccessorySelection) {
        beginRequest(selection)
        try {
            check(cancelPending()) { "Prior native permission cancellation is still running" }
            val accessory = inventory.resolve(selection)
            val intent = Intent(action).setPackage(app.packageName).setData(address(selection))
            val owned = Pending(selection, PendingIntent.getBroadcast(app, 0, intent,
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_CANCEL_CURRENT))
            synchronized(gate) { check(pending == null); pending = owned; live() }
            manager.requestPermission(accessory, owned.intent)
        } catch (failure: Exception) {
            failed(failure)
            try { cancelPending() } catch (cleanup: Exception) { failure.addSuppressed(cleanup) }
            throw failure
        } finally {
            try { if (synchronized(gate) { closed }) cleanup() }
            finally { synchronized(gate) { requestBusy = false; operations-- } }
        }
    }

    private fun beginComplete(owned: Pending): Boolean = synchronized(gate) {
        if (closed || error != null || pending !== owned || owned.selection.epoch != epoch) return false
        operations++
        true
    }
    private fun complete(owned: Pending) {
        if (!beginComplete(owned)) return
        try {
            val result = permission(owned.selection)
            if (!cancelPending(owned)) return
            if (synchronized(gate) { !closed && error == null && epoch == owned.selection.epoch }) {
                report(owned.selection, result.first, result.second)
            }
        } catch (failure: Exception) { failed(failure) }
        finally { synchronized(gate) { operations-- } }
    }
    private fun permission(selection: AccessorySelection): Pair<List<AccessoryDescription>, Boolean> = try {
        inventory.snapshot() to inventory.permitted(selection)
    } catch (_: Exception) { emptyList<AccessoryDescription>() to false }

    /** Keep the exact native token reachable until cancellation returns. */
    private fun cancelPending(expected: Pending? = null): Boolean {
        val owned = synchronized(gate) {
            if (cancelling) return false
            val value = pending ?: return true
            if (expected != null && value !== expected) return false
            cancelling = true
            value
        }
        try {
            owned.intent.cancel()
            synchronized(gate) { if (pending === owned) pending = null }
            return true
        } catch (failure: Exception) { failed(failure); throw failure }
        finally { synchronized(gate) { cancelling = false } }
    }
    private fun unregister() {
        synchronized(gate) {
            if (!registered || registering || unregistering) return
            unregistering = true
        }
        try { app.unregisterReceiver(receiver); synchronized(gate) { registered = false } }
        catch (failure: Exception) { failed(failure); throw failure }
        finally { synchronized(gate) { unregistering = false } }
    }
    private fun cleanup() {
        closeConnectionResources(listOf(Closeable { cancelPending() }, Closeable { unregister() }))
    }
    override fun close() {
        synchronized(gate) {
            closed = true
            if (closeBusy) return
            closeBusy = true; operations++
        }
        try { cleanup() }
        finally { synchronized(gate) { closeBusy = false; operations-- } }
    }
}
