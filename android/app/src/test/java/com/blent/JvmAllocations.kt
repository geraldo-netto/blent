package com.blent

import org.junit.Assert.assertTrue

/** JVM-only allocation observation; does not claim Android device counters. */
internal class JvmAllocations {
    private val bean = Class.forName("java.lang.management.ManagementFactory")
        .getMethod("getThreadMXBean").invoke(null)
    private val getter = Class.forName("com.sun.management.ThreadMXBean")
        .getMethod("getThreadAllocatedBytes", Long::class.javaPrimitiveType)
    fun read() = (getter.invoke(bean, Thread.currentThread().id) as Long).also { assertTrue(it >= 0) }
}
