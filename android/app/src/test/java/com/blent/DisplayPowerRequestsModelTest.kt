package com.blent

import org.junit.Assert.*
import org.junit.Test

class DisplayPowerRequestsModelTest {
    private enum class Step { OPEN_A, OPEN_B, PUBLISH_A, PUBLISH_B, CLOSE_A, CLOSE_B, RECEIVE, DUPLICATE }

    /** Independent finite mailbox model: numeric owner names, one queue slot,
     * one latest desired value and a separate dirty flag. No native resources. */
    private class Trace {
        val requests = DisplayPowerRequests()
        private val initial = requests.open(StreamingPower())
        private val foreign = DisplayPowerRequests().open(StreamingPower())
        private val owners = arrayOf(initial.owner, foreign.owner)
        private val names = intArrayOf(0, -1)
        private var allocated = 0
        private var current = 0
        private var live = true
        private var desired: StreamingPower? = StreamingPower()
        private var dirty = true
        private var queued: Int? = 0
        private var notice = initial.notice
        private var previous = checkNotNull(foreign.notice)
        private var revision = 0
        private val views = ArrayList<Pair<DisplayPowerRequest, Int>>()

        fun step(step: Step) {
            when (step) {
                Step.OPEN_A -> open(0)
                Step.OPEN_B -> open(1)
                Step.PUBLISH_A -> publish(0)
                Step.PUBLISH_B -> publish(1)
                Step.CLOSE_A -> close(0)
                Step.CLOSE_B -> close(1)
                Step.RECEIVE -> receive()
                Step.DUPLICATE -> duplicate()
            }
            assertState()
        }
        private fun open(slot: Int) {
            val power = StreamingPower(active = slot == 1)
            val result = requests.open(power)
            allocated++; current = allocated
            names[slot] = current; owners[slot] = result.owner
            live = true; desired = power; dirty = true; revision++
            admission(result.notice)
        }
        private fun publish(slot: Int) {
            val power = StreamingPower(batterySaver = slot == 1)
            val result = requests.publish(owners[slot], power)
            if (names[slot] == current && live) {
                desired = power; dirty = true; revision++
            }
            admission(result)
        }
        private fun close(slot: Int) {
            val result = requests.close(owners[slot])
            if (names[slot] == current && live) {
                live = false; desired = null; dirty = true; revision++
            }
            admission(result)
        }
        private fun admission(result: DisplayPowerNotice?) {
            if (queued != null || !dirty) { assertNull(result); return }
            val accepted = checkNotNull(result)
            assertEquals(desired, accepted.original.power)
            queued = current; notice = accepted
        }
        private fun receive() {
            val original = notice ?: previous
            val result = requests.receive(original)
            assertEquals(queued != null, result.accepted)
            if (queued == null) { assertNull(result.request); assertNull(result.next); return }
            checkRequest(result.request)
            previous = original; queued = null; notice = null
            admission(result.next)
        }
        private fun checkRequest(request: DisplayPowerRequest?) {
            if (queued != current) { assertNull(request); return }
            val accepted = checkNotNull(request)
            assertEquals(desired, accepted.power)
            dirty = false
            views.add(accepted to revision)
        }
        private fun duplicate() {
            val result = requests.receive(previous)
            assertFalse(result.accepted)
            assertNull(result.request); assertNull(result.next)
        }
        private fun assertState() {
            for (slot in 0..1) assertEquals(names[slot] == current && live, requests.owns(owners[slot]))
            for ((view, version) in views) assertEquals(version == revision, requests.current(view))
            assertEquals(queued == null, notice == null)
        }
    }

    @Test fun t1062All4096FourStepHistoriesPreserveOriginalOwnerAndOneNoticeAdmission() {
        repeat(4096) { encoded ->
            val trace = Trace()
            var remaining = encoded
            repeat(4) {
                trace.step(Step.entries[remaining % 8])
                remaining /= 8
            }
        }
    }
}
