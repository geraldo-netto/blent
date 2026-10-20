package com.blent.connection

/** Detached complete result. Null means neither identity nor persisted trust
 * exists; absence of this wrapper means execution supplied no complete result. */
internal class SavedTrustResult(val computers: SavedComputers?)

/** One bounded transaction's ordered pages. No native ownership or authority.
 * The enclosing worker serializes callbacks before execute returns. */
internal class SavedTrustCollector {
    private class Header(val local: ConnectionPeer?, val revision: Long, val total: Int)
    private var header: Header? = null
    private val peers = mutableListOf<ConnectionPeer>()

    fun add(page: SavedComputersPage) {
        val previous = header
        if (previous != null) {
            check(peers.size < previous.total) { "Duplicate completed saved-computer result" }
            require(previous.local == page.local && previous.revision == page.revision && previous.total == page.total)
        }
        require(page.offset == peers.size) { "Saved-computer page gap or replay" }
        val values = page.peers
        checkBoundary(values)
        header = previous ?: Header(page.local, page.revision, page.total)
        peers.addAll(values)
    }

    private fun checkBoundary(values: List<ConnectionPeer>) {
        val last = peers.lastOrNull() ?: return
        val first = values.firstOrNull() ?: return
        require(last.fingerprint < first.fingerprint) { "Unordered saved computers" }
    }

    fun result(): SavedTrustResult {
        val complete = checkNotNull(header) { "Missing saved-computer result" }
        check(peers.size == complete.total) { "Incomplete saved-computer result" }
        return SavedTrustResult(complete.local?.let { SavedComputers(it, complete.revision, peers) })
    }
}
