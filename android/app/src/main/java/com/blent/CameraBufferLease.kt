package com.blent

/** Single-owner encoded buffer lifetime. Claim release before calling native
 * code: an ambiguous native failure must never cause a second release attempt. */
internal class CameraBufferLease(private val release: () -> Unit) : AutoCloseable {
    private var owned = true
    override fun close() {
        if (!owned) return
        owned = false
        release()
    }
}
