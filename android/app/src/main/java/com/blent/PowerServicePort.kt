package com.blent

/** One actual foreground component. Posting queues a delivery callback, never invokes it inline;
 * it never acquires native wake resources. Stop is bound to this instance. */
internal interface PowerServicePort {
    fun post(notice: DisplayPowerNotice): Boolean
    fun stop()
}
