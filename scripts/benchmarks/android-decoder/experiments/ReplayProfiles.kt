package com.blent.benchmark
import com.blent.*
internal fun applyProfile(decoder: DecoderSession, name: String) {
    decoder.callbackFactory = if (name.startsWith("callback-")) replayCallbacks() else null
    decoder.profile = replayDriverProfile(name)
    decoder.formatCodec = replayFormatFactory(name)
}
internal fun replayDriverProfile(name: String): DecoderProfile = when (name) {
    "legacy", "socket-heap", "socket-direct" -> DecoderProfile()
    "render-latest" -> DecoderProfile(renderLatest = true)
    "sync-normal" -> DecoderProfile(renderPriority = Thread.NORM_PRIORITY)
    "callback-legacy" -> DecoderProfile()
    "callback-supported", "callback-supported1", "callback-unhinted" ->
        DecoderProfile(renderPriority = Thread.NORM_PRIORITY)
    else -> error("Unknown decoder profile")
}
internal fun replayFormatFactory(name: String): DecoderFormatFactory = when (name) {
    "callback-supported" -> { codec, parameters, hints -> ReplayDecoderConfiguration.format(codec, parameters, hints, 2) }
    "callback-supported1" -> { codec, parameters, hints -> ReplayDecoderConfiguration.format(codec, parameters, hints, 1) }
    "callback-unhinted" -> { _, parameters, hints -> DecoderConfiguration.baseFormat(parameters, hints) }
    else -> { _, parameters, hints -> DecoderConfiguration.format(parameters, hints) }
}
