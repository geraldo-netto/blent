package com.blent

/** Read-only, nonblocking, thread-safe relevance of one original desired view.
 * Invalid original views never revive; fresh views need fresh permits. Relevance
 * grants neither authentication nor native ownership/retirement proof. */
internal fun interface DisplayPowerPermit {
    fun current(): Boolean
}

/** Fixed policy has no external changing view; foreground identity still applies. */
internal object FixedDisplayPowerPermit : DisplayPowerPermit {
    override fun current() = true
}

/** One immutable policy and its original source-view relevance. */
internal data class DisplayPowerDemand(
    val power: StreamingPower,
    val permit: DisplayPowerPermit = FixedDisplayPowerPermit,
)
