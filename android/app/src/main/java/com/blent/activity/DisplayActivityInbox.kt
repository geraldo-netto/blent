package com.blent.activity

import org.json.JSONObject

internal sealed interface DisplayActivityState {
    data object Unavailable : DisplayActivityState
    data object Unsupported : DisplayActivityState
    data class Observed(val signal: DisplayActivitySignal) : DisplayActivityState
}

/** Immutable observation; source identity belongs to the original control inbox,
 * not its wire counters. Retaining it grants no media/native authority. */
internal class DisplayActivityUpdate(val source: Any, val state: DisplayActivityState)

/** One current observation, one opaque source identity and a terminal close bit.
 * The control owner serializes receive/close. UI reads only immutable snapshots. */
internal class DisplayActivityInbox {
    private val source = Any()
    @Volatile var update = DisplayActivityUpdate(source, DisplayActivityState.Unavailable)
        private set
    private var closed = false

    fun receive(message: JSONObject): Boolean = try {
        check(!closed) { "Original display activity source closed" }
        if (message.optString("status") == "connected") greeting(message)
        else if (message.has("display_activity")) accept(DisplayActivitySignal.read(message.getJSONObject("display_activity")))
        else false
    } catch (failure: Exception) { close(); throw failure }

    private fun greeting(message: JSONObject): Boolean {
        if (update.state !== DisplayActivityState.Unavailable) {
            if (message.has("display_activity")) return accept(DisplayActivitySignal.read(message.getJSONObject("display_activity")))
            check(update.state === DisplayActivityState.Unsupported) { "Original activity protocol withdrawn" }
            return false
        }
        val state = if (message.has("display_activity")) DisplayActivityState.Observed(DisplayActivitySignal.read(message.getJSONObject("display_activity")))
            else DisplayActivityState.Unsupported
        update = DisplayActivityUpdate(source, state)
        return true
    }
    private fun accept(next: DisplayActivitySignal): Boolean {
        val previous = (update.state as? DisplayActivityState.Observed)?.signal
            ?: error("Display activity protocol not announced")
        require(next.revision >= previous.revision) { "Stale display activity revision" }
        if (next.revision == previous.revision) {
            require(next == previous) { "Conflicting display activity revision" }
            return false
        }
        require(next.stopped >= previous.stopped) { "Display stop fence regressed" }
        require(!previous.active || next.active || next.stopped > previous.stopped) { "Missing original display stop fence" }
        update = DisplayActivityUpdate(source, DisplayActivityState.Observed(next))
        return true
    }
    fun close() {
        if (closed) return
        closed = true
        update = DisplayActivityUpdate(source, DisplayActivityState.Unavailable)
    }
}

/** Desired work only; callers separately validate the original authority and
 * actual native retirement. */
internal val DisplayActivityState.desired: Boolean
    get() = when (this) {
        DisplayActivityState.Unavailable -> false
        DisplayActivityState.Unsupported -> true
        is DisplayActivityState.Observed -> signal.active
    }
