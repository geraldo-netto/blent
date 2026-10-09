package com.blent

import org.junit.Assert.*
import org.junit.Test

class AudioIdentityTest {
    @Test fun t795_alignedIdentityTransfersRemovedArrayWithExactMetadataAndNoRemainingAlias() {
        var now = 100L
        val queue = AudioPlaybackQueue(20) { now }
        val first = ShortArray(960) { (it * 71).toShort() }
        val second = ShortArray(960) { (it * -29).toShort() }
        val expected = first.clone()
        queue.offer(AudioBlock(first, true))
        now++
        queue.offer(AudioBlock(second))
        val output = queue.poll()!!
        assertArrayEquals(expected, output.samples)
        assertEquals(100, output.at)
        assertTrue(output.discontinuity)
        assertSame("T795 identity path copied PCM", first, output.samples)
        output.samples.fill(0)
        val next = queue.poll()!!
        assertSame(second, next.samples)
        assertEquals(101, next.at)
        assertFalse(next.discontinuity)
        assertEquals((-29).toShort(), next.samples[1])
        assertNull(queue.poll())
    }

    @Test fun t795_fractionalOffsetAndSignedRatesMatchFlatScalarReference() {
        val random = java.util.Random(795)
        val values = ShortArray(960 * 3) { random.nextInt(65536).toShort() }
        for (trial in 0 until 128) {
            val rate = listOf(-1000, -500, 0, 500, 1000)[trial % 5]
            val phase = if (trial % 3 == 0) 0L else random.nextInt(1_000_000).toLong()
            val offset = if (trial % 4 == 0) 0 else random.nextInt(480)
            val queue = AudioPlaybackQueue(20) { 0 }
            val sources = (0..2).map { values.copyOfRange(it * 960, (it + 1) * 960) }
            sources.forEach { queue.offer(AudioBlock(it)) }
            fun field(name: String, value: Any) = AudioPlaybackQueue::class.java.getDeclaredField(name)
                .apply { isAccessible = true }.set(queue, value)
            field("ppm", rate); field("phase", phase); field("offset", offset)
            val output = queue.poll()!!
            val expected = ShortArray(960) { sample ->
                val position = offset * 1_000_000L + phase + (sample / 2) * (1_000_000L + rate)
                val left = (position / 1_000_000).toInt() * 2 + sample % 2
                val fraction = position % 1_000_000
                val a = values[left].toLong()
                (a + (values[left + 2] - a) * fraction / 1_000_000).toShort()
            }
            assertArrayEquals("T795 rate=$rate phase=$phase offset=$offset", expected, output.samples)
            if (rate == 0 && phase == 0L && offset == 0) assertSame(sources[0], output.samples)
            else assertTrue(sources.none { it === output.samples })
            output.samples.fill(0)
            assertTrue("T795 partial input array escaped", sources.drop(1).all { it.any { v -> v != 0.toShort() } })
        }
    }

    @Test fun t795_overflowExpiryDiscontinuityAndInvalidLengthsRemainBounded() {
        var now = 0L
        val queue = AudioPlaybackQueue(20) { now }
        for (size in listOf(0, 1, 959, 961, 1920)) {
            assertThrows(IllegalArgumentException::class.java) { queue.offer(AudioBlock(ShortArray(size))) }
        }
        val blocks = List(25) { ShortArray(960) { _ -> it.toShort() } }
        for (block in blocks) { now++; queue.offer(AudioBlock(block)) }
        val output = queue.poll()!!
        assertSame(blocks[5], output.samples)
        assertTrue(output.discontinuity)
        now += 201
        assertNull(queue.poll())
        queue.offer(AudioBlock(blocks[0], true)); queue.offer(AudioBlock(blocks[1]))
        assertSame(blocks[0], queue.poll()!!.samples)
        now--
        assertThrows(IllegalArgumentException::class.java) { queue.poll() }
    }
}
