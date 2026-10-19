package com.blent.connection

/** One private-process transaction; neither variant selects USB or grants media. */
internal sealed interface SavedTrustRequest {
    data object List : SavedTrustRequest
    data class Forget(val selected: SavedComputerSelection) : SavedTrustRequest
}

/** At most 22 bounded pages for 1024 peers, below the existing 32-event queue.
 * A null local identity means no identity AND no persisted trust exist. */
internal class SavedComputersPage(
    val local: ConnectionPeer?, val revision: Long, val total: Int, val offset: Int,
    peers: List<ConnectionPeer>,
) {
    companion object { const val CAPACITY = 48 }
    private val values = boundedPeerCopy(peers, 48)
    val peers: List<ConnectionPeer> get() = values.toList()
    init { positions(); identity(); order() }
    private fun positions() {
        require(total in 0..1024 && offset in 0..total)
        require(offset % CAPACITY == 0)
        require(total == 0 || offset < total)
        require(values.size == minOf(CAPACITY, total - offset))
    }
    private fun identity() {
        require(revision >= 0)
        if (local == null) require(revision == 0L && total == 0)
        if (revision == 0L) require(total == 0)
    }
    private fun order() {
        require(values.zipWithNext().all { (left, right) -> left.fingerprint < right.fingerprint })
    }
}

/** Typed local launch description. The Android adapter supplies Binder framing. */
internal sealed interface ConnectionProcessLaunch {
    val retirementTimeoutMs: Long
    data class Connect(val spec: ConnectionWorkerSpec) : ConnectionProcessLaunch {
        override val retirementTimeoutMs: Long get() = spec.io.retirementTimeoutMs
    }
    data class Trust(val request: SavedTrustRequest) : ConnectionProcessLaunch {
        override val retirementTimeoutMs: Long get() = 5000
    }
}
