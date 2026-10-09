package com.blent

/** A request, not evidence that Android honored a hint or rendered a frame. */
internal data class DecoderSelection(
    val name: String, val codec: String, val profile: String, val level: Int, val depth: Int,
    val lowLatency: Boolean, val operatingRate: Int?,
) {
    /** Identifies supplied configuration keys, not whether Android honored them. */
    fun receipt(allowHints: Boolean = true): String {
        val low = if (allowHints && lowLatency) 1 else 0
        val rate = if (allowHints) operatingRate ?: 0 else 0
        return "${name.length}:$name:$codec:$profile:$level:$depth:$low:$rate"
    }

}
