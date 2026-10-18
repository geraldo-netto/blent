package com.blent

/** A fresh unadmitted handle. The factory returns ownership before acquisition
 * and owns any partial factory failure. Acquire may leave unknown obligations.
 * Retire returns normally only after this exact handle's grant/callbacks have
 * settled or proven not started; errors never authorize release/reuse.
 * Native/backend evidence is the implementation's responsibility. */
internal interface WakeHandle {
    fun acquire()
    fun retire()
}
