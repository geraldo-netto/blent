package com.blent

import org.junit.Assert.*
import org.junit.Test

class DisplayPowerRequestsTest {
    @Test fun t1062OriginalQueuedNoticeCannotApplyReplacementPolicyAndDuplicateCannotAcknowledgeIt() {
        val requests = DisplayPowerRequests()
        val old = requests.open(StreamingPower())
        val replacement = requests.open(StreamingPower(true))
        assertNull(replacement.notice)
        assertFalse(requests.owns(old.owner)); assertTrue(requests.owns(replacement.owner))
        val delivery = requests.receive(checkNotNull(old.notice))
        assertTrue(delivery.accepted)
        assertNull("T1062 old notice applied a replacement foreground owner", delivery.request)
        val next = checkNotNull(delivery.next)
        assertSame(replacement.owner, next.original.owner)
        assertFalse(requests.receive(old.notice).accepted)
        val current = checkNotNull(requests.receive(next).request)
        assertSame(replacement.owner, current.owner)
        assertTrue(requests.current(current))
        assertEquals(StreamingPower(true), current.power)
    }

    @Test fun t1062CloseIsTerminalForOwnerButItsExactStopReceiptRemainsCurrent() {
        val requests = DisplayPowerRequests()
        val original = requests.open(StreamingPower())
        val active = checkNotNull(requests.receive(checkNotNull(original.notice)).request)
        val stop = checkNotNull(requests.close(original.owner))
        assertFalse(requests.owns(original.owner)); assertFalse(requests.current(active))
        assertNull(requests.publish(original.owner, StreamingPower()))
        assertNull(requests.close(original.owner))
        val closed = checkNotNull(requests.receive(stop).request)
        assertNull(closed.power); assertTrue(requests.current(closed))
        assertNull(requests.receive(stop).request)
        requests.open(StreamingPower())
        assertFalse(requests.current(closed))
    }

    @Test fun t1062CurrentOwnerCoalescesThousandsOfChangesWithoutNewQueuedAdmission() {
        val requests = DisplayPowerRequests()
        val original = requests.open(StreamingPower())
        repeat(1000) { assertNull(requests.publish(original.owner, StreamingPower(active = it % 2 == 0))) }
        val delivery = requests.receive(checkNotNull(original.notice))
        val observed = checkNotNull(delivery.request)
        assertFalse(checkNotNull(observed.power).active)
        assertNull(delivery.next)
        requests.publish(original.owner, StreamingPower(true))
        assertFalse(requests.current(observed))
    }

    @Test fun t1062ForeignOwnersAndNoticesNeverConsumeAnotherOriginalAdmission() {
        val requests = DisplayPowerRequests()
        val original = requests.open(StreamingPower())
        val foreign = DisplayPowerRequests().open(StreamingPower())
        assertFalse(requests.owns(foreign.owner))
        assertNull(requests.close(foreign.owner)); assertNull(requests.publish(foreign.owner, StreamingPower()))
        assertFalse(requests.receive(checkNotNull(foreign.notice)).accepted)
        assertNotNull(requests.receive(checkNotNull(original.notice)).request)
    }
}
