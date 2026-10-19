package com.blent.connection

import java.nio.channels.OverlappingFileLockException

/** Process-local admission before opening any child of the original directory.
 * Native fileKey identities are borrowed immutable values. Each slot is owned
 * until both original channels actually close; uncertain cleanup retains it for
 * the containing private process's lifetime. The OS lock still excludes other
 * processes. No timer, GC callback or logical completion releases a slot. */
internal class TrustDirectoryOwnership(private val capacity: Int = 64) {
    init { require(capacity in 1..1024) }
    private val owners = mutableMapOf<Any, Any>()

    @Synchronized fun reserve(key: Any): Lease {
        if (owners.containsKey(key)) throw OverlappingFileLockException()
        check(owners.size < capacity) { "Trust directory ownership capacity exhausted" }
        val token = Any()
        val lease = Lease(this, key, token)
        owners[key] = token
        return lease
    }

    @Synchronized private fun release(key: Any, token: Any) {
        check(owners[key] === token)
        owners.remove(key)
    }

    class Lease internal constructor(
        private val owner: TrustDirectoryOwnership,
        private val key: Any,
        private val token: Any,
    ) : AutoCloseable {
        private var released = false
        @Synchronized override fun close() {
            if (released) return
            owner.release(key, token)
            released = true
        }
    }
}
