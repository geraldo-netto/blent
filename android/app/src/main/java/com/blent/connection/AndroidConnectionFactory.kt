package com.blent.connection

import android.content.Context
import android.hardware.usb.UsbManager
import android.os.SystemClock
import java.nio.file.FileAlreadyExistsException
import java.nio.file.Files
import java.nio.file.LinkOption.NOFOLLOW_LINKS
import java.nio.file.Path
import java.nio.file.attribute.PosixFileAttributes
import java.nio.file.attribute.PosixFilePermissions

/** Android composition only. Neither construction nor create touches native
 * services/storage; every supplier runs on the retained setup authority thread. */
internal class AndroidConnectionFactory(
    private val context: () -> Context,
    private val clock: () -> Long = SystemClock::elapsedRealtime,
    private val loadIdentity: (Path) -> ClientTlsIdentity = ::loadAndroidConnectionIdentity,
    private val loadTrust: (Path) -> PeerTrustStore = { path -> AndroidPeerTrustStore.open(path) },
) {
    companion object { val profile = AccessoryProfile("Blent", "Blent", "1") }

    fun create(spec: ConnectionWorkerSpec, report: (ConnectionWorkerEvent) -> Unit): ConnectionWorker {
        val selected = AccessorySelection.parse(spec.device)
        return AndroidConnectionWorker(spec, { current, wake ->
            val backend = AndroidConnectionSetupBackend(
                { opening(spec, selected, current) }, { loadIdentity(directory()) },
            )
            ConnectionSetupOwner(backend, { loadTrust(directory()) }, clock, spec.setupLimits(), wake)
        }, report, clock)
    }

    internal fun opening(spec: ConnectionWorkerSpec, selected: AccessorySelection, current: () -> Boolean): AndroidAccessoryOpening {
        val manager = checkNotNull(context().getSystemService(Context.USB_SERVICE) as? UsbManager) { "USB accessory service unavailable" }
        val inventory = AndroidAccessoryInventory(manager, profile, spec.accessoryCapacity)
        return AndroidAccessoryOpening(inventory, selected, current)
    }

    internal fun directory(): Path {
        val path = context().noBackupFilesDir.toPath().resolve("blent-connection")
        val permissions = PosixFilePermissions.fromString("rwx------")
        try { Files.createDirectory(path, PosixFilePermissions.asFileAttribute(permissions)) }
        catch (_: FileAlreadyExistsException) { /* Validate existing state; never repair it silently. */ }
        val attributes = Files.readAttributes(path, PosixFileAttributes::class.java, NOFOLLOW_LINKS)
        require(attributes.isDirectory && attributes.permissions() == permissions) { "Connection directory must be private" }
        return path
    }
}

internal fun loadAndroidConnectionIdentity(directory: Path): ClientTlsIdentity =
    AndroidConnectionIdentity(directory.toFile(), AndroidIdentityStore.open(), AndroidIdentityGenerator()).load()
