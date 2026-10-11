package com.blent

import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test

class DecoderMailboxDefaultTest {
    private fun input(data: ByteArray = byteArrayOf(7)) =
        DecoderInput(data, 0, data.size, true, 0xffff_ffffL)

    @Test fun t967DefaultClockOwnsFramesAndBoundsAdmission() {
        val mailbox = DecoderMailbox()
        try {
            val bytes = byteArrayOf(3, 7, 9)
            val before = System.nanoTime()
            assertTrue(mailbox.offer(DecoderInput(bytes, 1, 1, true, 42)))
            val after = System.nanoTime()
            val first = mailbox.first()!!
            val budget = TimeUnit.MILLISECONDS.toNanos(200)
            assertTrue(first.deadline - before >= budget)
            assertTrue(first.deadline - after <= budget)
            bytes.fill(0)
            assertArrayEquals(byteArrayOf(7), first.input.data)
            assertEquals(0, first.input.offset)
            assertTrue(first.input.configuration)
            assertEquals(42L, first.input.sequence)
            assertTrue(mailbox.offer(input()))
            val blockedAt = System.nanoTime()
            assertFalse(mailbox.offer(input()))
            val waited = System.nanoTime() - blockedAt
            assertTrue(waited >= budget)
            assertTrue(waited < TimeUnit.MILLISECONDS.toNanos(1500))
            mailbox.complete(first.copy())
            assertSame(first, mailbox.first())
            mailbox.complete(first)
            assertTrue(mailbox.offer(input()))
            mailbox.complete(mailbox.first()!!)
            mailbox.complete(mailbox.first()!!)
            assertNull(mailbox.first())
        } finally { mailbox.close() }
        assertTrue(mailbox.closed)
        assertFalse(mailbox.offer(input()))
        assertNull(mailbox.first())
    }

    @Test fun t967InvalidSizeAndOffsetReleaseReservedCapacity() {
        val mailbox = DecoderMailbox()
        try {
            for (size in listOf(Int.MIN_VALUE, -1, 0, VideoReceiver.MAX_FRAME_SIZE + 1, Int.MAX_VALUE)) {
                assertThrows(IllegalArgumentException::class.java) { mailbox.offer(input().copy(size = size)) }
                assertNull(mailbox.first())
            }
            for (offset in listOf(Int.MIN_VALUE, -1, 2, Int.MAX_VALUE)) {
                assertThrows(RuntimeException::class.java) { mailbox.offer(input().copy(offset = offset)) }
                assertTrue(mailbox.offer(input()))
                mailbox.complete(mailbox.first()!!)
                assertNull(mailbox.first())
            }
            assertTrue(mailbox.offer(input()))
            assertTrue(mailbox.offer(input()))
        } finally { mailbox.close() }
        assertNull(mailbox.first())
    }
}
