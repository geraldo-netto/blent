package com.blent.connection

import android.content.Context
import java.security.KeyStore
import java.nio.file.Path

/** Lazy private-process composition. Browsing never generates a key, opens USB
 * or invokes a pairing/media factory. Existing stores use the same lock adapter. */
internal class AndroidSavedTrustFactory(
    private val context: () -> Context,
    private val identity: () -> ConnectionPeer? = { currentSavedTrustIdentity() },
    private val store: (Path) -> PeerTrustStore = { AndroidPeerTrustStore.open(it) },
) {
    fun create(request: SavedTrustRequest, report: (ConnectionWorkerEvent) -> Unit): ConnectionWorker =
        SavedTrustWorker(request, ::open, report)
    private fun open(): SavedTrustAccess {
        val directory = AndroidConnectionFactory(context).directory()
        return SavedTrustAccesses.open(store(directory), identity)
    }
}

/** Read existing public certificate only. No identity repair or key generation. */
internal fun currentSavedTrustIdentity(factory: (String) -> KeyStore = KeyStore::getInstance): ConnectionPeer? {
    val store = factory("AndroidKeyStore")
    store.load(null, null)
    val certificate = store.getCertificate(AndroidConnectionIdentity.ALIAS) ?: return null
    return ConnectionIdentity(certificate.encoded).fingerprint()
}
