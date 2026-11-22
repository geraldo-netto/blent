package com.blent

import android.media.MediaCodec
import java.nio.ByteBuffer

/** Replay transport borrows native input storage. The caller closes its reader
 * on retirement so an outstanding fill can return and settle its native borrow. */
internal fun DecoderSession.feedReplayInput(codec: MediaCodec, info: DecoderInputInfo,
                                          fill: (ByteBuffer) -> Unit): Boolean = useInput(codec) { input ->
    if (input.asynchronous) false else input.queue(info) { buffer ->
        fill(buffer)
        check(!input.retired) { "Codec retired during input read" }
        input.arrived(info, System.nanoTime())
    }
}
