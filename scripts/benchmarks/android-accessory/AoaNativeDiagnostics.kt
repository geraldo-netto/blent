package com.blent

import android.os.ParcelFileDescriptor
import android.system.Os
import android.system.OsConstants
import com.blent.connection.AndroidConnectionIdentity
import com.blent.connection.ClientTlsIdentity
import java.net.Socket
import java.security.SecureRandom
import java.security.Signature
import javax.net.ssl.SSLContext
import javax.net.ssl.X509KeyManager

/** Profile-only observations. Duplicated socket descriptors are closed here;
 * socket ownership never changes. No key material or signature bytes are logged. */
internal object AoaNativeDiagnostics {
    fun socket(socket: Socket): String = runCatching {
        checkNotNull(ParcelFileDescriptor.fromSocket(socket)).use { duplicate ->
            val fd = duplicate.fileDescriptor
            val stat = Os.fstat(fd)
            "socket class=${socket.javaClass.name} connected=${socket.isConnected} closed=${socket.isClosed} " +
                "duplicateFd=${duplicate.fd} mode=${stat.st_mode} inode=${stat.st_ino} socketMode=${OsConstants.S_ISSOCK(stat.st_mode)} " +
                "local=${Os.getsockname(fd)} peer=${Os.getpeername(fd)} target=${Os.readlink("/proc/self/fd/${duplicate.fd}")}"
        }
    }.getOrElse { "Socket diagnostic failed: ${AoaSecureDiagnostics.describe(it)}" }

    fun provider(): String = runCatching {
        val context = SSLContext.getInstance("TLS")
        "TLS selection context=${context.javaClass.name} provider=${context.provider.name} providerClass=${context.provider.javaClass.name}"
    }.getOrElse { "TLS provider diagnostic failed: ${AoaSecureDiagnostics.describe(it)}" }

    /** Conscrypt signs its already hashed handshake with NONEwithECDSA. This
     * uses the existing opaque key and changes no aliases or authorizations. */
    fun rawSigning(identity: ClientTlsIdentity): String = runCatching {
        val manager = identity.keyManagers().single() as X509KeyManager
        val key = checkNotNull(manager.getPrivateKey(AndroidConnectionIdentity.ALIAS))
        val certificate = checkNotNull(manager.getCertificateChain(AndroidConnectionIdentity.ALIAS)).single()
        val challenge = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val signer = Signature.getInstance("NONEwithECDSA").apply { initSign(key); update(challenge) }
        val signature = signer.sign()
        val verifier = Signature.getInstance("NONEwithECDSA").apply { initVerify(certificate); update(challenge) }
        check(verifier.verify(signature)) { "Raw signature did not verify" }
        "Existing opaque key supports native TLS NONEwithECDSA signing; keyClass=${key.javaClass.name} provider=${signer.provider.name}"
    }.getOrElse { "Existing opaque key raw TLS signing failed: ${AoaSecureDiagnostics.describe(it)}" }
}
