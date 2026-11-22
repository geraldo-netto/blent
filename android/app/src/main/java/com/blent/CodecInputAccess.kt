package com.blent

import java.nio.ByteBuffer

/** One input admission borrows its original native owner. No buffer is retained
 * here; native release waits for queue's complete dequeue/fill/submit operation. */
internal class CodecInputAccess(
    private val owner: DecoderCodecAccess,
    private val consumer: DecoderInputConsumer?,
    private val arrival: (Int, Long) -> Unit,
    private val queued: () -> Unit,
) : DecoderInputAccess {
    override val asynchronous: Boolean get() = consumer != null
    override val retired: Boolean get() = owner.retired

    override fun arrived(input: DecoderInputInfo, nanos: Long) {
        if (!input.configuration) arrival(input.sequence.toInt(), nanos)
    }

    override fun offer(input: DecoderInput): Boolean {
        if (consumer != null) {
            check(consumer.offer(input) || owner.retired) { "Decoder callback input queue stalled" }
            return true
        }
        return queue(input, input::copyTo)
    }

    override fun queue(input: DecoderInputInfo, fill: (ByteBuffer) -> Unit): Boolean {
        if (asynchronous) return false
        val submitted = queueSlot { index ->
            queueCodecInput(owner.codec, index, input) { buffer ->
                fill(buffer)
                check(!owner.retired) { "Codec retired during input fill" }
            }
        }
        if (submitted && !input.configuration) queued()
        return submitted
    }

    private fun queueSlot(submit: (Int) -> Unit): Boolean {
        repeat(10) {
            val submitted = owner.use {
                val index = owner.codec.dequeueInputBuffer(20_000)
                if (index < 0 || owner.retired) false
                else { submit(index); true }
            } ?: return false
            if (submitted) return true
            if (owner.retired) return false
        }
        throw IllegalStateException("Decoder input unavailable after ten 20ms waits")
    }
}
