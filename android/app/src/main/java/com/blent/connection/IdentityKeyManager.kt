package com.blent.connection

import java.net.Socket
import java.security.Principal
import java.security.PrivateKey
import java.security.cert.X509Certificate
import javax.net.ssl.SSLEngine
import javax.net.ssl.X509ExtendedKeyManager

/** Constrains JSSE to this client identity; unrelated application keys stay private. */
internal class IdentityKeyManager(
    private val alias: String,
    private val key: PrivateKey,
    private val certificate: X509Certificate,
) : X509ExtendedKeyManager() {
    private fun accepts(keyType: String?, issuers: Array<out Principal>?): Boolean =
        keyType == "EC" && (issuers.isNullOrEmpty() || issuers.contains(certificate.issuerX500Principal))

    override fun getClientAliases(keyType: String?, issuers: Array<out Principal>?): Array<String>? =
        if (accepts(keyType, issuers)) arrayOf(alias) else null

    override fun chooseClientAlias(keyType: Array<out String>?, issuers: Array<out Principal>?, socket: Socket?): String? =
        if (keyType?.any { accepts(it, issuers) } == true) alias else null

    override fun chooseEngineClientAlias(keyType: Array<out String>?, issuers: Array<out Principal>?, engine: SSLEngine?): String? =
        chooseClientAlias(keyType, issuers, null)

    override fun getCertificateChain(requested: String?): Array<X509Certificate>? =
        if (requested == alias) arrayOf(certificate) else null

    override fun getPrivateKey(requested: String?): PrivateKey? = if (requested == alias) key else null
    override fun getServerAliases(keyType: String?, issuers: Array<out Principal>?): Array<String>? = null
    override fun chooseServerAlias(keyType: String?, issuers: Array<out Principal>?, socket: Socket?): String? = null
    override fun chooseEngineServerAlias(keyType: String?, issuers: Array<out Principal>?, engine: SSLEngine?): String? = null
}
