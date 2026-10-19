package com.blent.connection

/** Exclusive native trust access. Implementations close partial resources on
 * failure; the enclosing private process sponsors any unconfirmed cleanup. */
internal interface SavedTrustAccess : AutoCloseable {
    fun snapshot(): SavedComputers?
    fun forget(selected: SavedComputerSelection): Boolean
}

internal object SavedTrustAccesses {
    fun open(store: PeerTrustStore, identity: () -> ConnectionPeer?): SavedTrustAccess {
        val local = try { identity() } catch (failure: Exception) { abandon(store, failure) }
        if (local != null) return Loaded(ConnectionTrust(store, local, 1024))
        try { check(store.load() == null) { "Saved trust exists without the tablet identity; preserve storage" } }
        catch (failure: Exception) { abandon(store, failure) }
        return Empty(store)
    }
    private fun abandon(store: PeerTrustStore, failure: Exception): Nothing {
        try { store.close() } catch (cleanup: Exception) { failure.addSuppressed(cleanup) }
        throw failure
    }
    private class Loaded(private val trust: ConnectionTrust) : SavedTrustAccess {
        override fun snapshot(): SavedComputers = trust.snapshot()
        override fun forget(selected: SavedComputerSelection): Boolean = trust.forget(ConnectionPairing(), selected)
        override fun close() = trust.close()
    }
    private class Empty(private val store: PeerTrustStore) : SavedTrustAccess {
        private var closed = false
        override fun snapshot(): SavedComputers? { check(!closed); return null }
        override fun forget(selected: SavedComputerSelection): Boolean { check(!closed); error("Tablet identity is not initialized") }
        override fun close() { if (!closed) { closed = true; store.close() } }
    }
}
