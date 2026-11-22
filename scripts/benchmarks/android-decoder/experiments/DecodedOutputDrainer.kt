package com.blent

import android.media.MediaCodec

/** Experimental decode-all/render-latest policy. Never drops compressed input.
 * Called inside one codec lifetime borrow, with no receiver monitor held. */
internal class DecodedOutputDrainer(
    private val codec: MediaCodec,
    private val alive: () -> Boolean,
    private val discarded: (Int) -> Unit,
) : DecoderOutputPolicy {
    private val nextInfo = MediaCodec.BufferInfo()

    override fun release(index: Int, sequence: Int): Int? {
        var pendingIndex = index
        var pendingSequence = sequence
        repeat(3) { // At most four outputs per batch: no starvation.
            if (!alive()) return null
            val next = codec.dequeueOutputBuffer(nextInfo, 0)
            if (next < 0) return present(pendingIndex, pendingSequence)
            if (!alive()) return null
            codec.releaseOutputBuffer(pendingIndex, false)
            discarded(pendingSequence)
            pendingIndex = next
            pendingSequence = nextInfo.presentationTimeUs.toInt()
        }
        return present(pendingIndex, pendingSequence)
    }

    private fun present(index: Int, sequence: Int): Int? {
        if (!alive()) return null
        codec.releaseOutputBuffer(index, true)
        return sequence
    }
}
