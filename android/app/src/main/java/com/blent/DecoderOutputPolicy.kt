package com.blent

/** Called inside the original native output borrow. Return the sequence actually
 * presented, or null after retirement; never retain a borrowed native buffer. */
internal fun interface DecoderOutputPolicy {
    fun release(index: Int, sequence: Int): Int?
}
