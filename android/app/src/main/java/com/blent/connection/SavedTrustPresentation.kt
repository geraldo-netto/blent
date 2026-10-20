package com.blent.connection

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

internal enum class SavedTrustPhase { IDLE, BUSY, RETIRING, READY, FAILED, QUARANTINED }

/** Heap-only display state; fresh viewer and request identities never wrap. */
internal data class SavedTrustView(
    val viewer: Any? = null,
    val request: Any? = null,
    val phase: SavedTrustPhase = SavedTrustPhase.IDLE,
    val computers: SavedComputers? = null,
    val cancelled: Boolean = false,
    val failure: String? = null,
    val cleanup: String? = null,
)

internal class SavedTrustChoice internal constructor(
    internal val viewer: Any,
    internal val snapshot: SavedComputers,
    val selected: SavedComputerSelection,
)

/** No native work. The application binding owns accepted requests independently
 * of a UI attachment. Reducer callbacks never acquire the binding action lock. */
internal class SavedTrustPresentation {
    private val mutable = MutableStateFlow(SavedTrustView())
    val state = mutable.asStateFlow()

    @Synchronized fun attach(): Any {
        val viewer = Any()
        mutable.value = mutable.value.copy(viewer = viewer)
        return viewer
    }
    @Synchronized fun detach(viewer: Any) {
        if (mutable.value.viewer === viewer) mutable.value = mutable.value.copy(viewer = null)
    }
    @Synchronized fun canRequest(viewer: Any): Boolean {
        val value = mutable.value
        if (value.viewer !== viewer) return false
        return value.phase !in setOf(SavedTrustPhase.BUSY, SavedTrustPhase.RETIRING, SavedTrustPhase.QUARANTINED)
    }
    @Synchronized fun begin(viewer: Any): Any? {
        if (!canRequest(viewer)) return null
        val token = Any()
        mutable.value = SavedTrustView(viewer = viewer, request = token, phase = SavedTrustPhase.BUSY)
        return token
    }
    @Synchronized fun choice(viewer: Any, peer: ConnectionPeer): SavedTrustChoice? {
        val value = mutable.value
        if (value.viewer !== viewer || value.phase != SavedTrustPhase.READY) return null
        val snapshot = value.computers ?: return null
        if (!snapshot.peers.contains(peer)) return null
        return SavedTrustChoice(viewer, snapshot, snapshot.select(peer))
    }
    @Synchronized fun selection(viewer: Any, choice: SavedTrustChoice): SavedComputerSelection? {
        if (choice.viewer !== viewer || !canRequest(viewer)) return null
        val value = mutable.value
        if (value.phase != SavedTrustPhase.READY || value.computers !== choice.snapshot) return null
        if (!value.computers.peers.contains(choice.selected.peer)) return null
        val actual = value.computers.select(choice.selected.peer)
        return choice.selected.takeIf { it == actual }
    }
    @Synchronized fun consume(token: Any, accept: () -> Unit) {
        val value = mutable.value
        if (value.request === token && value.phase == SavedTrustPhase.BUSY) accept()
    }
    @Synchronized fun stop(viewer: Any): Boolean {
        val value = mutable.value
        if (value.viewer !== viewer || value.phase != SavedTrustPhase.BUSY) return false
        mutable.value = value.copy(phase = SavedTrustPhase.RETIRING, cancelled = true,
            cleanup = "Stopping saved-computer access…")
        return true
    }
    @Synchronized fun ended(token: Any, failure: Exception?) {
        val value = mutable.value
        if (value.request !== token) return
        mutable.value = value.copy(phase = SavedTrustPhase.RETIRING,
            failure = failure?.message ?: failure?.javaClass?.simpleName,
            cleanup = "Closing saved-computer access…")
    }
    @Synchronized fun retired(token: Any, result: SavedTrustResult?) {
        val value = mutable.value
        if (value.request !== token) return
        val failure = finalFailure(value, result)
        mutable.value = value.copy(phase = if (failure == null) SavedTrustPhase.READY else SavedTrustPhase.FAILED,
            computers = if (failure == null) result?.computers else null, failure = failure, cleanup = null)
    }
    private fun finalFailure(value: SavedTrustView, result: SavedTrustResult?): String? {
        if (value.cancelled) return "Stopped. Refresh to check the saved list; a forget already accepted by storage may have completed."
        if (value.failure != null) return value.failure
        return if (result == null) "No complete saved-computer result. Refresh to try again." else null
    }
    @Synchronized fun cleanup(token: Any, text: String, failed: Boolean, stalled: Boolean) {
        val value = mutable.value
        if (value.request !== token) return
        if (failed) mutable.value = value.copy(phase = SavedTrustPhase.QUARANTINED, cleanup = text)
        else if (stalled && value.phase == SavedTrustPhase.RETIRING) mutable.value = value.copy(cleanup = text)
    }
}
