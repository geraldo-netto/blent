package com.blent.connection

import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first

/** Activity-owned retry policy; no native resource or permission is acquired here.
 * One loop retains one proven peer/attempt and one bounded delay. The restart
 * port rechecks identity and ResourceLane retains original native settlement.
 * Full presentation values and route credentials never cross a retry suspension. */
internal class RememberedConnection(
    private val state: StateFlow<ConnectionView>,
    private val restart: suspend (Any) -> Any?,
) {
    private data class Target(val attempt: Any, val peer: ConnectionPeer)
    private data class Ended(val allowed: Boolean, val authenticated: Boolean)

    suspend fun run() {
        var observed: Any? = null
        while (true) {
            val initial = awaitEligible(observed)
            observed = initial.attempt
            follow(initial)
        }
    }

    private suspend fun awaitEligible(observed: Any?): Target {
        val value = state.first { eligible(it) && it.attempt !== observed }
        return Target(checkNotNull(value.attempt), checkNotNull(value.retryPeer))
    }

    private suspend fun follow(initial: Target) {
        var attempt = initial.attempt
        var pause = 2000L
        while (true) {
            val ended = awaitEnd(attempt, initial.peer)
            if (!ended.allowed) return
            if (ended.authenticated) pause = 2000L
            delay(pause)
            if (!matchesAttempt(attempt, state.value, initial.peer)) return
            attempt = restart(attempt) ?: return
            pause = (pause * 2).coerceAtMost(30000L)
        }
    }

    private suspend fun awaitEnd(attempt: Any, peer: ConnectionPeer): Ended {
        var authenticated = false
        val value = state.first {
            if (it.attempt === attempt && it.retryPeer == peer) authenticated = true
            !it.active || !matchesAttempt(attempt, it, peer)
        }
        return Ended(matchesAttempt(attempt, value, peer), authenticated)
    }

    private fun eligible(value: ConnectionView): Boolean =
        value.attempt != null && value.reconnectAllowed && value.retryPeer != null

    private fun matchesAttempt(expected: Any, value: ConnectionView, peer: ConnectionPeer): Boolean =
        value.attempt === expected && value.reconnectAllowed && samePeer(value, peer)

    private fun samePeer(value: ConnectionView, peer: ConnectionPeer): Boolean {
        val selected = value.peer ?: return true
        return selected.ticket.peer == peer && value.remembered != false
    }
}
