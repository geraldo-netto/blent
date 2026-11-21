package com.blent.connection

import java.nio.file.Path

/** Filesystem adapter for an existing application-private directory excluded from
 * backup. All I/O runs inside a supervised worker. The supplied directory must
 * remain owned by that worker; arbitrary external paths are not accepted by UI. */
internal class AndroidPeerTrustStore private constructor(private val files: TrustFileAccess) : PeerTrustStore {
    private var failed = false
    private var closed = false

    companion object {
        fun open(directory: Path): AndroidPeerTrustStore = open(NioTrustFileAccess.open(directory))
        fun open(files: TrustFileAccess): AndroidPeerTrustStore {
            try {
                files.lock()
                return AndroidPeerTrustStore(files)
            } catch (failure: Exception) {
                try { files.close() } catch (cleanup: Exception) { failure.addSuppressed(cleanup) }
                throw failure
            }
        }
    }

    private fun ready() {
        check(!closed && !failed)
        TrustPublication(files).recover()
    }

    override fun load(): ByteArray? {
        try {
            ready()
            return files.read("trust.bin", ConnectionTrustCodec.MAX_RECORD)
        } catch (failure: Exception) { failed = true; throw failure }
    }

    override fun replace(record: ByteArray) {
        try {
            ready()
            TrustPublication(files).replace(record)
        } catch (failure: Exception) { failed = true; throw failure }
    }

    /** Explicit offline reset; keeps the keystore identity and stable lock. */
    fun reset() {
        check(!closed && !failed)
        try { TrustPublication(files).reset() }
        catch (failure: Exception) { failed = true; throw failure }
    }

    override fun close() {
        if (closed) return
        closed = true
        files.close()
    }
}

/** Injectable native operations. writeNew synchronizes bytes before returning;
 * replace performs an atomic rename. Errors propagate; no cleanup removes evidence. */
internal interface TrustFileAccess : AutoCloseable {
    fun lock()
    fun exists(name: String): Boolean
    fun read(name: String, maximum: Int): ByteArray?
    fun writeNew(name: String, bytes: ByteArray)
    fun replace(source: String, target: String)
    fun syncDirectory()
    fun remove(name: String)
}
