package com.blent

import android.content.Context
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.os.ParcelFileDescriptor
import android.os.Process
import java.io.Closeable
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference
import kotlin.concurrent.thread

/** Native open/read/write/close never run on the activity thread. One run per
 * activity; a stuck operation can terminate only the dedicated probe process. */
internal class AoaProbeRun(
    context: Context, private val manager: UsbManager, private val accessory: UsbAccessory,
    private val report: (String) -> Unit,
) {
    companion object { private val admitted = AtomicBoolean() }
    private val resultFile = File(context.filesDir, "aoa-probe-result.txt")
    private val stopped = AtomicBoolean()
    private val finished = AtomicBoolean()
    private val ports = AtomicReference<Ports?>()
    fun start(): Boolean {
        if (!admitted.compareAndSet(false, true)) return false
        thread(name = "aoa-probe-watchdog", isDaemon = true) {
            Thread.sleep(60000)
            if (!finished.get()) Process.killProcess(Process.myPid())
        }
        thread(name = "aoa-probe-io") { execute() }
        return true
    }

    fun stop() {
        if (!stopped.compareAndSet(false, true)) return
        thread(name = "aoa-probe-stop", isDaemon = true) { runCatching { ports.get()?.close() } }
        thread(name = "aoa-probe-retirement", isDaemon = true) {
            Thread.sleep(1000)
            Process.killProcess(Process.myPid())
        }
    }

    private fun execute() {
        val result = runCatching { exchange() }.fold(
            { "PASS: $it payload bytes echoed; ${AoaProbeWire.MAX_PACKETS} checked frames." },
            { "Probe ended: ${it.javaClass.simpleName}: ${it.message}" },
        )
        // If native close is stuck, the watchdog remains responsible for exit.
        val closed = runCatching { ports.getAndSet(null)?.close() }
        if (closed.isFailure) { stop(); return }
        resultFile.writeText(result + "\nSource: " + ProfileBuild.SOURCE_ID + "\n")
        report(result + "\nUnplug/replug after testing. Re-enable USB debugging only when returning to ADB.")
        finished.set(true)
    }

    private fun exchange(): Long {
        check(!stopped.get())
        val descriptor = checkNotNull(manager.openAccessory(accessory)) { "Accessory open rejected" }
        val owned = Ports.open(descriptor)
        ports.set(owned)
        check(!stopped.get())
        return AoaProbeWire.echo(owned.input, owned.output) { frames, bytes ->
            report("USB test: $frames frames; $bytes payload bytes echoed.")
        }
    }

    private class Ports(
        val input: ParcelFileDescriptor.AutoCloseInputStream,
        val output: ParcelFileDescriptor.AutoCloseOutputStream,
    ) : Closeable {
        private var closed = false
        @Synchronized override fun close() {
            if (closed) return
            try { input.close() } finally { output.close() }
            closed = true
        }
        companion object {
            fun open(descriptor: ParcelFileDescriptor): Ports {
                var duplicate: ParcelFileDescriptor? = null
                try {
                    duplicate = ParcelFileDescriptor.dup(descriptor.fileDescriptor)
                    val input = ParcelFileDescriptor.AutoCloseInputStream(duplicate)
                    return Ports(input, ParcelFileDescriptor.AutoCloseOutputStream(descriptor))
                } catch (error: Exception) {
                    try { duplicate?.close() } finally { descriptor.close() }
                    throw error
                }
            }
        }
    }
}
