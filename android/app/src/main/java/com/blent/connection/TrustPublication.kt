package com.blent.connection

/** Borrows the store's exclusive lock for one bounded publication/recovery.
 * A complete intent contains the new snapshot, including revoked approvals.
 * Invalid intents fail closed. Only an explicitly requested reset may erase them. */
internal class TrustPublication(private val files: TrustFileAccess) {
    fun recover() {
        if (files.exists("trust.reset")) {
            files.read("trust.reset", 0)
            files.syncDirectory()
            finishReset()
            return
        }
        val intended = files.read("trust.pending", ConnectionTrustCodec.MAX_RECORD)
        if (intended != null) {
            ConnectionTrustCodec.decode(intended)
            files.syncDirectory()
            publish(intended)
        }
        check(!files.exists("trust.tmp")) { "Interrupted trust publication; use Reset saved computers" }
    }

    fun replace(record: ByteArray) {
        ConnectionTrustCodec.decode(record)
        files.writeNew("trust.pending", record)
        files.syncDirectory()
        publish(record)
    }

    private fun publish(record: ByteArray) {
        removeIfPresent("trust.tmp")
        files.writeNew("trust.tmp", record)
        files.replace("trust.tmp", "trust.bin")
        files.syncDirectory()
        files.remove("trust.pending")
        files.syncDirectory()
    }

    fun reset() {
        if (!files.exists("trust.reset")) files.writeNew("trust.reset", byteArrayOf())
        files.read("trust.reset", 0)
        files.syncDirectory()
        finishReset()
    }

    private fun finishReset() {
        for (name in listOf("trust.bin", "trust.tmp", "trust.pending")) removeIfPresent(name)
        files.syncDirectory()
        files.remove("trust.reset")
        files.syncDirectory()
    }

    private fun removeIfPresent(name: String) {
        if (files.exists(name)) files.remove(name)
    }
}
