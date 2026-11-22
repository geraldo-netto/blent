package com.blent

import java.nio.ByteBuffer

/** Input operations are scoped to the original admitted decoder. Buffers may be
 * borrowed only inside queue's fill call and must not escape. Async consumers
 * detach packets before offer returns; they do not admit direct buffer fills. */
internal interface DecoderInputAccess {
    val asynchronous: Boolean
    val retired: Boolean
    fun arrived(input: DecoderInputInfo, nanos: Long)
    fun offer(input: DecoderInput): Boolean
    fun queue(input: DecoderInputInfo, fill: (ByteBuffer) -> Unit): Boolean
}
