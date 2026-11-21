package com.blent

import org.junit.Assert.*
import org.junit.Test

class AudioRecoveryTest {
    @Test fun t1584AllTargetsKeepOneJitterBlockAndBoundSurplus() {
        for (target in 2..20) {
            for (count in target..25) {
                val queue = AudioPlaybackQueue(target * 10) { 0 }
                repeat(count) { index -> queue.offer(AudioBlock(ShortArray(960) { index.toShort() })) }
                val capacity = minOf(count, 20)
                val retained = if (capacity > target + 1) target else capacity
                val output = queue.poll()!!
                assertEquals(count - retained, output.samples[0].toInt())
                assertEquals(retained != count, output.discontinuity)
            }
        }
    }

    @Test fun t1584RepeatedStallsReturnToTargetWithOneDiscontinuity() {
        var now = 0L
        val queue = AudioPlaybackQueue(40) { now }
        var sequence = 0
        repeat(3) {
            repeat(6) {
                now = sequence * 10L
                queue.offer(AudioBlock(ShortArray(960) { sequence.toShort() }))
                sequence++
            }
            now += 10
            val output = queue.poll()!!
            assertEquals("T1584 stale playback backlog", sequence - 4, output.samples[0].toInt())
            assertTrue(output.discontinuity)
            val next = queue.poll()!!
            assertEquals(sequence - 3, next.samples[0].toInt())
            assertFalse(next.discontinuity)
        }
    }
}
