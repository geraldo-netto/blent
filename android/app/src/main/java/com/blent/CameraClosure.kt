package com.blent

/** One original device. The worker waits; the camera callback handler only signals. */
internal class CameraClosure<T : Any>(private val requestClose: (T) -> Unit) {
    private val lock = java.lang.Object()
    private var original: T? = null
    private var requested = false
    private var retired = false

    fun bind(value: T) = synchronized(lock) {
        check(original == null || original === value) { "Camera closure identity changed" }
        original = value
    }

    fun completed(value: T) = synchronized(lock) {
        if (original !== value) return@synchronized
        retired = true
        lock.notifyAll()
    }

    fun close(value: T) {
        val request = synchronized(lock) {
            check(original === value) { "Camera closure identity mismatch" }
            (!requested).also { requested = true }
        }
        if (request) requestClose(value)
        synchronized(lock) { while (!retired) lock.wait() }
    }
}
