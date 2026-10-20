package com.blent

import java.util.concurrent.atomic.AtomicLong

/** Process-local callback generation; exhaustion is terminal, never wrapping. */
internal class WorkerGenerations(initial: Long = 0) {
    private val value = AtomicLong(initial)
    init { require(initial >= 0) }
    fun next(): Long {
        while (true) {
            val previous = value.get()
            check(previous < Long.MAX_VALUE) { "Worker generations exhausted" }
            if (value.compareAndSet(previous, previous + 1)) return previous + 1
        }
    }
}

