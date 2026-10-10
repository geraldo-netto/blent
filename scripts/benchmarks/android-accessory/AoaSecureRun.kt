package com.blent

import android.content.Context
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.os.Process
import android.os.SystemClock
import android.util.Log
import com.blent.connection.*
import java.io.DataInputStream
import java.io.File
import java.net.Socket
import java.security.MessageDigest
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference
import kotlin.concurrent.thread

/** T846 profile-only native TLS qualification. Synthetic bytes, no media authority,
 * no saved peer trust. Dedicated process bounds native provider/USB shutdown. */
internal class AoaSecureRun(
    context: Context, private val manager: UsbManager, private val accessory: UsbAccessory,
    private val report: (String) -> Unit,
) {
    companion object { private val admitted = AtomicBoolean() }
    private val directory = context.filesDir
    private val stopped = AtomicBoolean()
    private val finished = AtomicBoolean()
    private val awaitingApproval = AtomicBoolean()
    private val approval = CountDownLatch(1)
    private val bridge = AtomicReference<SocketBridge?>()
    private val stopOrigin = AtomicReference<String?>()

    fun start(): Boolean {
        if (!admitted.compareAndSet(false, true)) return false
        thread(name = "aoa-secure-watchdog", isDaemon = true) {
            Thread.sleep(180000)
            if (!finished.get()) Process.killProcess(Process.myPid())
        }
        thread(name = "aoa-secure-test") { execute() }
        return true
    }

    fun approve() {
        if (awaitingApproval.compareAndSet(true, false)) {
            report("Tablet approval accepted. Waiting for computer approval, then mutual TLS.")
            approval.countDown()
        }
    }

    fun stop() {
        if (!stopped.compareAndSet(false, true)) return
        val origin = "${SystemClock.elapsedRealtime()} Stop requested on ${Thread.currentThread().name}: ${AoaSecureDiagnostics.ownedStack(Throwable())}"
        stopOrigin.set(origin)
        Log.i("BlentAoaSecure", origin) // UI-safe: no application file I/O here.
        approval.countDown()
        thread(name = "aoa-secure-close", isDaemon = true) { runCatching { bridge.get()?.close() } }
        thread(name = "aoa-secure-retirement", isDaemon = true) {
            Thread.sleep(1000)
            Process.killProcess(Process.myPid())
        }
    }

    private fun execute() {
        runCatching { File(directory, "aoa-secure-progress.txt").writeText("Source: ${ProfileBuild.SOURCE_ID}\n") }
        val exchanged = runCatching { exchange() }
        exchanged.exceptionOrNull()?.let {
            val failure = "Exchange failed before cleanup: ${AoaSecureDiagnostics.describe(it)}"
            record(failure)
            record("Owned failure stack:\n${AoaSecureDiagnostics.ownedStack(it)}")
            stopOrigin.get()?.let { origin -> record(origin) }
            bridge.get()?.failure()?.let { cause -> record("Bridge failure before cleanup: ${AoaSecureDiagnostics.describe(cause)}") }
            report(failure)
            runCatching { File(directory, "aoa-secure-result.txt").writeText("$failure\nCleanup pending\nSource: ${ProfileBuild.SOURCE_ID}\n") }
        }
        val retired = runCatching { retireBridge() }
        val result = AoaSecureDiagnostics.render(exchanged, retired)
        record(result)
        File(directory, "aoa-secure-result.txt").writeText(result + "\nSource: " + ProfileBuild.SOURCE_ID + "\n")
        report(result + "\nTap Stop, then unplug/replug after testing.")
        if (retired.isFailure) stop() else finished.set(true)
    }

    private fun record(message: String) {
        val line = "${SystemClock.elapsedRealtime()} $message"
        Log.i("BlentAoaSecure", line)
        runCatching { File(directory, "aoa-secure-progress.txt").appendText("$line\n") }
    }

    private fun stage(message: String) { record(message); report(message) }

    private fun retireBridge() {
        val owned = bridge.get() ?: return
        stage("Closing native bridge resources")
        owned.close()
        stage("Waiting for actual native bridge thread retirement")
        repeat(100) {
            if (owned.retired()) return
            Thread.sleep(10)
        }
        error("Native bridge retirement incomplete")
    }

    private fun exchange(): Long {
        check(!stopped.get())
        stage("Loading AndroidKeyStore client identity")
        val identity = AndroidConnectionIdentity(directory, AndroidIdentityStore.open(), AndroidIdentityGenerator()).load()
        stage("Opening USB accessory descriptor")
        val descriptor = checkNotNull(manager.openAccessory(accessory)) { "Accessory open rejected" }
        stage("Duplicating USB descriptor for input and output")
        val ports = AndroidAccessoryPorts.open(descriptor)
        stage("Creating private loopback TLS bridge")
        val owned = SocketBridgeAdmission().open(ports.input, ports.output)
        bridge.set(owned)
        check(!stopped.get())
        stage("Starting USB bridge workers")
        owned.start()
        val socket = authenticate(owned.socket, identity)
        val bytes = socket.use {
            stage("Transferring authenticated synthetic frames")
            try { echo(it) } finally { stage("Closing TLS socket and sending close_notify") }
        }
        stage("Waiting for TLS output bridge to drain and retire")
        check(owned.awaitOutputRetired(3000)) { "TLS output did not retire after complete drain" }
        return bytes
    }

    private fun authenticate(socket: Socket, identity: ClientTlsIdentity): Socket {
        socket.soTimeout = 60000
        stage("Reading computer public identity")
        val input = DataInputStream(socket.getInputStream())
        val header = ByteArray(ConnectionHello.HEADER_BYTES).also(input::readFully)
        val peer = ByteArray(ConnectionHello.decode(header, true)).also(input::readFully)
        val local = identity.publicIdentity().certificate()
        stage("Sending tablet public identity")
        socket.getOutputStream().apply {
            write(ConnectionHello.encode(false, local.size)); write(local); flush()
        }
        val policy = ConnectionPairing()
        val peerId = ConnectionPeer.parse(fingerprint(peer))
        val begun = SystemClock.elapsedRealtime()
        val ticket = policy.begin(peerId, false, begun, 120000)
        stage("Waiting for tablet fingerprint approval")
        awaitingApproval.set(true)
        report("Compare fingerprints with computer:\nComputer: ${peerId.fingerprint}\nTablet: ${fingerprint(local)}\nMatch Computer with Computer and Tablet with Tablet on the host. Computer and Tablet are different identities. If each corresponding label matches, tap Approve displayed computer. Then approve on host. No trust is saved.")
        check(approval.await(120, TimeUnit.SECONDS) && !stopped.get()) { "Approval cancelled or expired" }
        policy.approve(ticket, false, SystemClock.elapsedRealtime())
        ready(socket, begun + 120000)
        stage("Inspecting existing opaque key and socket before TLS context")
        record(AoaNativeDiagnostics.socket(socket))
        record(AoaNativeDiagnostics.provider())
        record(AoaNativeDiagnostics.rawSigning(identity))
        stage("Creating stock TLS context with approved peer certificate")
        val tls = SecureConnection(identity.keyManagers(), peer)
        stage("Starting mutual TLS handshake and authenticated protocol preface")
        record(AoaNativeDiagnostics.socket(socket))
        val secured = tls.connect(socket)
        try {
            stage("Checking current approval after verified TLS")
            check(!stopped.get())
            val lease = policy.authenticated(ticket, peerId, SystemClock.elapsedRealtime())
            check(policy.current(lease))
            report("Authenticated USB test running; six synthetic channels.")
            return secured
        } catch (failure: Exception) { secured.close(); throw failure }
    }

    private fun ready(socket: Socket, deadline: Long) {
        check(!stopped.get())
        val remaining = deadline - SystemClock.elapsedRealtime()
        check(remaining in 1..120000) { "Approval expired before readiness" }
        socket.soTimeout = remaining.toInt()
        stage("Sending tablet approval-ready marker")
        socket.getOutputStream().apply { write(ConnectionReady.encode(false)); flush() }
        stage("Waiting for computer approval-ready marker")
        val marker = ByteArray(ConnectionReady.BYTES)
        DataInputStream(socket.getInputStream()).readFully(marker)
        ConnectionReady.decode(marker, true)
        check(SystemClock.elapsedRealtime() < deadline) { "Approval expired during readiness" }
        check(!stopped.get())
    }

    private fun fingerprint(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256")
        .digest(bytes).joinToString("") { "%02x".format(it.toInt() and 255) }

    private fun echo(socket: Socket): Long {
        socket.soTimeout = 10000
        return AoaSecureEcho(stopped = stopped::get, report = report)
            .exchange(socket.getInputStream(), socket.getOutputStream())
    }
}
