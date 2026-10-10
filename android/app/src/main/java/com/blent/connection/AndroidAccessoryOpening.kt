package com.blent.connection

import android.os.ParcelFileDescriptor
import java.io.Closeable
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean

/** One worker-owned native opening. Keep this owner even when open throws.
 * A separate supervised closer calls close; stopped late acquisitions are closed
 * by their opener. Retirement includes open completion and actual bridge exit. */
internal class AndroidAccessoryOpening(
    private val inventory: AndroidAccessoryInventory,
    private val selection: AccessorySelection,
    private val current: () -> Boolean,
    private val admission: SocketBridgeAdmission = SocketBridgeAdmission(),
    private val ports: (ParcelFileDescriptor) -> AndroidAccessoryPorts = { AndroidAccessoryPorts.open(it) },
) : ConnectionSetupLifetime {
    private val gate = Any()
    private val started = AtomicBoolean()
    private val finished = AtomicBoolean()
    private val closing = AtomicBoolean()
    private val closed = AtomicBoolean()
    private val lateCloseFailed = AtomicBoolean()
    private var resource: Closeable? = null
    @Volatile private var bridge: SocketBridge? = null

    private fun live() {
        check(!closing.get() && current()) { "Accessory selection is no longer current" }
        check(inventory.permitted(selection)) { "Accessory permission is required" }
    }
    private fun <T : Closeable> adopt(value: T): T {
        synchronized(gate) {
            if (!closing.get()) { resource = value; return value }
        }
        try { value.close() } catch (failure: Exception) { lateCloseFailed.set(true); throw failure }
        error("Accessory opening completed after cancellation")
    }
    fun open(): Socket {
        check(started.compareAndSet(false, true)) { "Accessory opening already admitted" }
        try {
            live()
            val descriptor = adopt(inventory.open(selection))
            live()
            val streams = adopt(ports(descriptor))
            live()
            val created = admission.open(streams.input, streams.output)
            bridge = created
            adopt(created)
            live()
            created.start()
            live()
            return created.socket
        } finally { finished.set(true) }
    }
    override fun close() {
        if (!closing.compareAndSet(false, true)) return
        val owned = synchronized(gate) { resource }
        owned?.close()
        closed.set(true)
    }
    override fun retired(): Boolean = (!started.get() || finished.get()) && closed.get() &&
        !lateCloseFailed.get() && bridge?.retired() != false
}
