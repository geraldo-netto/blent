package com.blent.connection

import org.json.JSONObject
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.nio.file.attribute.PosixFilePermissions

/** T868 disposable app_process qualification. Never opens identity or production
 * trust paths. Argument: existing profile application's private no_backup dir. */
object TrustStoreProbe {
    @JvmStatic fun main(args: Array<String>) {
        require(args.size == 1)
        val parent = Paths.get(args[0])
        val root = Files.createTempDirectory(parent, "t868-",
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        val result = JSONObject().put("sdk", android.os.Build.VERSION.SDK_INT)
        try {
            validate(root)
            result.put("status", "PASS")
            result.put("checks", "remember,restart,revoke,exclusive-lock,pending-quarantine,temp-quarantine")
        } catch (failure: Exception) {
            result.put("status", "FAIL")
            result.put("error", "${failure.javaClass.name}: ${failure.message}".take(512))
        } finally {
            result.put("cleanup", root.toFile().deleteRecursively())
        }
        Files.write(parent.resolve("t868-result.json"), result.toString().toByteArray(Charsets.UTF_8))
        println(result)
        check(result.getString("status") == "PASS" && result.getBoolean("cleanup"))
    }

    private fun validate(root: Path) {
        val local = ConnectionPeer.parse("0".repeat(64))
        val peer = ConnectionPeer.parse("1".repeat(64))
        val pairing = ConnectionPairing()
        val ticket = pairing.begin(peer, false, 0, 100)
        pairing.approve(ticket, true, 1)
        val lease = pairing.authenticated(ticket, peer, 2)
        ConnectionTrust(AndroidPeerTrustStore.open(root), local).use { trust ->
            check(!trust.trusted(peer))
            check(trust.remember(pairing, lease))
            rejected { AndroidPeerTrustStore.open(root).close() }
        }
        ConnectionTrust(AndroidPeerTrustStore.open(root), local).use { trust ->
            check(trust.trusted(peer))
            check(trust.revoke(pairing, peer))
            check(!pairing.current(lease) && !trust.trusted(peer))
        }
        ConnectionTrust(AndroidPeerTrustStore.open(root), local).use { check(!it.trusted(peer)) }
        quarantine(root, "trust.pending")
        quarantine(root, "trust.tmp")
    }

    private fun quarantine(root: Path, name: String) {
        val path = root.resolve(name)
        Files.createFile(path, PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rw-------")))
        AndroidPeerTrustStore.open(root).use { store -> rejected { store.load() } }
        check(Files.exists(path))
        Files.delete(path) // Explicit repair of this disposable test fixture only.
    }

    private fun rejected(body: () -> Unit) {
        var failed = false
        try { body() } catch (_: Exception) { failed = true }
        check(failed) { "Invalid ownership or interrupted state accepted" }
    }
}
