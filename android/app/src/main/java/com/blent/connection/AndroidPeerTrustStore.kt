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
        check(!files.exists("trust.pending") && !files.exists("trust.tmp")) {
            "Interrupted trust publication; preserve storage for explicit recovery"
        }
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
            ConnectionTrustCodec.decode(record)
            files.writeNew("trust.pending", byteArrayOf(1))
            files.syncDirectory()
            files.writeNew("trust.tmp", record)
            files.replace("trust.tmp", "trust.bin")
            files.syncDirectory()
            files.remove("trust.pending")
            files.syncDirectory()
        } catch (failure: Exception) { failed = true; throw failure }
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
