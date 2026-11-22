package com.blent.connection

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import org.junit.Assert.*
import org.junit.Test

class RustProcessTest {
    @Test fun t1770InterruptedWaitRetainsExactProcessUntilReaped() {
        var waits = 0
        var killed = false
        val process = object : Process() {
            override fun waitFor(): Int {
                waits++
                if (waits == 1) throw InterruptedException("T1770 cancellation")
                check(killed)
                return 137
            }
            override fun destroyForcibly(): Process { killed = true; return this }
            override fun destroy() { killed = true }
            override fun exitValue() = 137
            override fun getInputStream() = ByteArrayInputStream(byteArrayOf())
            override fun getErrorStream() = ByteArrayInputStream(byteArrayOf())
            override fun getOutputStream() = ByteArrayOutputStream()
        }
        try {
            assertEquals(137, awaitRustProcess(process))
            assertEquals(2, waits); assertTrue(killed)
            assertTrue(Thread.currentThread().isInterrupted)
        } finally { Thread.interrupted() }
    }
}
