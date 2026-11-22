package com.blent

import android.media.MediaCodec

/** Native adapters captured together when a decoder startup is admitted. */
internal data class DecoderOutputDriver(
    val thread: (Runnable) -> Thread,
    val policy: (MediaCodec, () -> Boolean, (Int) -> Unit) -> DecoderOutputPolicy,
)

internal fun defaultDecoderOutputDriver() = DecoderOutputDriver(
    ::decoderRenderThread, { codec, alive, _ -> PresentDecodedOutput(codec, alive) })

private fun decoderRenderThread(body: Runnable) = Thread(body, "blent-render").apply { priority = Thread.MAX_PRIORITY }

/** Production releases every decoded frame, with no lookahead or input drop. */
internal class PresentDecodedOutput(private val codec: MediaCodec, private val alive: () -> Boolean) : DecoderOutputPolicy {
    override fun release(index: Int, sequence: Int): Int? {
        if (!alive()) return null
        codec.releaseOutputBuffer(index, true)
        return sequence
    }
}
