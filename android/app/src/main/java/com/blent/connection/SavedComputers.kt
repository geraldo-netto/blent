package com.blent.connection

/** Detached presentation values; no pairing, TLS proof or native capability.
 * Revision zero represents an empty store that has never been published. */
internal class SavedComputers(
    val local: ConnectionPeer,
    val revision: Long,
    peers: List<ConnectionPeer>,
) {
    private val values = boundedPeerCopy(peers, 1024)
    val peers: List<ConnectionPeer> get() = values.toList()
    init {
        require(revision >= 0 && values.size <= 1024)
        require(revision != 0L || values.isEmpty())
        require(values.zipWithNext().all { (left, right) -> left.fingerprint < right.fingerprint })
    }

    fun select(peer: ConnectionPeer): SavedComputerSelection {
        require(values.contains(peer)) { "Computer is not in the displayed saved list" }
        return SavedComputerSelection(local, revision, peer)
    }
}

/** The private-process adapter must additionally bind this conditional request
 * to its exact original client/worker generation. Never replay on reconnect. */
internal data class SavedComputerSelection(
    val local: ConnectionPeer,
    val revision: Long,
    val peer: ConnectionPeer,
) {
    init { require(revision > 0) }
}
