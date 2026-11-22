package com.blent.benchmark

import com.blent.*

internal fun replayOutputDriver(name: String): DecoderOutputDriver {
    val selected = defaultDecoderOutputDriver()
    return when (name) {
        "legacy", "socket-heap", "socket-direct", "callback-legacy" -> selected
        "render-latest" -> selected.copy(policy = ::DecodedOutputDrainer)
        "sync-normal", "callback-supported", "callback-supported1", "callback-unhinted" ->
            selected.copy(thread = { body -> Thread(body, "blent-render").apply { priority = Thread.NORM_PRIORITY } })
        else -> error("Unknown decoder profile")
    }
}
