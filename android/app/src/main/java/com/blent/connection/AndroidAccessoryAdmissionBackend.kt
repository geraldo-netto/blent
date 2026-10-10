package com.blent.connection

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbManager
import android.os.Build
import java.io.Closeable

/** Main-process Android boundary. Constructor is inert; all native operations
 * run on the admission worker/closer. Receiver callbacks only invalidate or
 * enqueue local state. Retain this object through failed and late acquisition. */
internal class AndroidAccessoryAdmissionBackend(
    private val context: () -> Context,
    profile: AccessoryProfile,
    capacity: Int = 32,
    private val inventoryFactory: (UsbManager) -> AndroidAccessoryInventory = { AndroidAccessoryInventory(it, profile, capacity) },
    private val permissionFactory: (Context, UsbManager, AndroidAccessoryInventory,
        (() -> Unit) -> Unit, (AccessorySelection, List<AccessoryDescription>, Boolean) -> Unit) -> AndroidAccessoryPermission =
        { app, manager, inventory, dispatch, report -> AndroidAccessoryPermission(app, manager, inventory, dispatch, report) },
) : AccessoryAdmissionBackend {
    init { require(capacity in 1..1024) }
    private val gate = Any()
    private var started = false
    private var ready = false
    private var closed = false
    private var closing = false
    private var registering = false
    private var registered = false
    private var unregistering = false
    private var uncertain = false
    private var operations = 0
    private var error: Exception? = null
    private var app: Context? = null
    private var inventory: AndroidAccessoryInventory? = null
    private var permission: AndroidAccessoryPermission? = null
    private var detached: (() -> Unit)? = null
    private var detachedSeen = false
    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action != UsbManager.ACTION_USB_ACCESSORY_DETACHED) return
            val callback = synchronized(gate) {
                if (closed || detachedSeen) return
                detachedSeen = true
                detached
            }
            try { callback?.invoke() } catch (failure: Exception) { failed(failure) }
        }
    }
    private fun failed(failure: Exception) { synchronized(gate) { if (error == null) error = failure } }
    private fun live() { check(!closed && !detachedSeen && error == null) { "Accessory admission is closed or failed" } }
    private fun register(app: Context) {
        synchronized(gate) { live(); registering = true; registered = true }
        try {
            val filter = IntentFilter(UsbManager.ACTION_USB_ACCESSORY_DETACHED)
            if (Build.VERSION.SDK_INT >= 33) app.registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED)
            else app.registerReceiver(receiver, filter)
        } finally { synchronized(gate) { registering = false } }
    }
    private fun adopt(value: AndroidAccessoryPermission) {
        synchronized(gate) { permission = value; uncertain = false; live() }
    }
    override fun start(dispatch: (() -> Unit) -> Unit,
        report: (AccessorySelection, List<AccessoryDescription>, Boolean) -> Unit, detached: () -> Unit) {
        synchronized(gate) { live(); check(!started); started = true; operations++; this.detached = detached }
        try {
            val context = context().applicationContext
            synchronized(gate) { app = context; live() }
            val manager = checkNotNull(context.getSystemService(UsbManager::class.java))
            val found = inventoryFactory(manager)
            synchronized(gate) { inventory = found; live() }
            register(context)
            synchronized(gate) { live(); uncertain = true }
            val owned = permissionFactory(context, manager, found, dispatch, report)
            adopt(owned)
            owned.start()
            synchronized(gate) { live(); ready = true }
        } catch (failure: Exception) { failed(failure); throw failure }
        finally {
            try { if (synchronized(gate) { closed }) cleanup() }
            finally { synchronized(gate) { operations-- } }
        }
    }
    private fun <T> operation(action: () -> T): T {
        synchronized(gate) { live(); check(ready); operations++ }
        try { return action() }
        catch (failure: Exception) { failed(failure); throw failure }
        finally { synchronized(gate) { operations-- } }
    }
    override fun snapshot(): List<AccessoryDescription> = operation { checkNotNull(inventory).snapshot() }
    override fun permitted(selection: AccessorySelection): Boolean = operation { checkNotNull(inventory).permitted(selection) }
    override fun request(selection: AccessorySelection) = operation { checkNotNull(permission).request(selection) }
    override fun failure(): Exception? {
        val owned = synchronized(gate) { error to permission }
        return owned.first ?: owned.second?.failure()
    }
    private fun unregister() {
        val context = synchronized(gate) {
            if (!registered || registering || unregistering) return
            unregistering = true
            checkNotNull(app)
        }
        try { context.unregisterReceiver(receiver); synchronized(gate) { registered = false } }
        finally { synchronized(gate) { unregistering = false } }
    }
    private fun cleanup() {
        val owned = synchronized(gate) { permission }
        closeConnectionResources(listOf(Closeable { owned?.close() }, Closeable { unregister() }))
    }
    override fun close() {
        synchronized(gate) {
            closed = true
            if (closing) return
            closing = true; operations++
        }
        try { cleanup() }
        catch (failure: Exception) { failed(failure); throw failure }
        finally { synchronized(gate) { closing = false; operations-- } }
    }
    override fun retired(): Boolean = synchronized(gate) {
        closed && operations == 0 && !registered && !registering && !uncertain && permission?.retired() != false
    }
}
