package com.blent

import android.media.MediaCodec
import android.os.Handler

/** Native-adapter borrow contract. Each successful use holds the original codec
 * until its body returns; closed admission rejects new operations. Only the
 * lifetime owner may retire or destroy native storage. */
internal interface DecoderCodecAccess {
    val codec: MediaCodec
    val retired: Boolean
    fun <T> use(block: () -> T): T?
}

internal typealias DecoderCallbackFactory = (DecoderCodecAccess, Handler, DecoderInputEvents) -> DecoderInputConsumer
