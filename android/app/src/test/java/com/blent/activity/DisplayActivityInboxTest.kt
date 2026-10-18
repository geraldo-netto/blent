package com.blent.activity

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayActivityInboxTest {
    private fun message(revision: Long, stopped: Long, active: Boolean, status: String = "display_activity") =
        JSONObject().put("status", status).put("display_activity", JSONObject().put("version", 1).put("revision", revision).put("stopped", stopped).put("active", active))
    @Test fun t1057OriginalSourceRetainsCoalescedStopAndExactDuplicateIsNoop() {
        val inbox = DisplayActivityInbox()
        assertTrue(inbox.receive(message(1, 0, false, "connected")))
        val source = inbox.update.source
        assertTrue(inbox.receive(message(2, 0, true)))
        assertTrue(inbox.receive(message(3, 1, true)))
        val resumed = inbox.update
        assertSame(source, resumed.source)
        assertEquals(DisplayActivitySignal(3, 1, true), (resumed.state as DisplayActivityState.Observed).signal)
        assertFalse(inbox.receive(message(3, 1, true)))
        assertSame(resumed, inbox.update)
        assertTrue(inbox.receive(message(4, 1, true)))
        assertFalse(inbox.receive(JSONObject().put("status", "mode")))
    }
    @Test fun t1057MalformedStaleAndUnannouncedUpdatesCloseOnlyOriginalSource() {
        for (invalid in listOf(message(2, 1, true), message(3, 1, false), message(4, 0, true), message(4, 1, false))) {
            val inbox = DisplayActivityInbox()
            inbox.receive(message(3, 1, true, "connected"))
            val original = inbox.update
            assertTrue(runCatching { inbox.receive(invalid) }.isFailure)
            assertSame(DisplayActivityState.Unavailable, inbox.update.state)
            assertNotSame(original, inbox.update)
            assertTrue(runCatching { inbox.receive(message(5, 2, true, "connected")) }.isFailure)
        }
        val inbox = DisplayActivityInbox()
        assertTrue(runCatching { inbox.receive(message(1, 0, true)) }.isFailure)
        assertSame(DisplayActivityState.Unavailable, inbox.update.state)
    }
    @Test fun t1057UnsupportedAnnouncementCannotSwitchProtocolAndClosedOwnerCannotRestart() {
        val inbox = DisplayActivityInbox()
        val legacy = JSONObject().put("status", "connected")
        assertTrue(inbox.receive(legacy))
        assertSame(DisplayActivityState.Unsupported, inbox.update.state)
        assertFalse(inbox.receive(legacy))
        assertTrue(runCatching { inbox.receive(message(1, 0, true)) }.isFailure)
        val supported = DisplayActivityInbox()
        supported.receive(message(1, 0, true, "connected"))
        assertTrue(runCatching { supported.receive(legacy) }.isFailure)
        supported.close(); supported.close()
        val replacement = DisplayActivityInbox()
        replacement.receive(message(1, 0, true, "connected"))
        assertNotSame(supported.update.source, replacement.update.source)
        assertSame(DisplayActivityState.Unavailable, supported.update.state)
    }
    private fun legalStep(revision: Long, stopped: Long, active: Boolean): Boolean {
        if (revision == 1L) return stopped == 0L && active
        return revision > 1 && stopped >= 0 && stopped <= revision && (active || stopped > 0)
    }
    @Test fun t1057BoundedInvalidCounterTransitionsCannotReviveOriginalObservation() {
        for (revision in listOf(Long.MIN_VALUE, -1, 0, 1, 2, 3, Long.MAX_VALUE)) {
            for (stopped in listOf(Long.MIN_VALUE, -1, 0, 1, 2, Long.MAX_VALUE)) {
                for (active in listOf(false, true)) {
                    val inbox = DisplayActivityInbox()
                    inbox.receive(message(1, 0, true, "connected"))
                    assertEquals(legalStep(revision, stopped, active), runCatching { inbox.receive(message(revision, stopped, active)) }.isSuccess)
                }
            }
        }
    }
    @Test fun t1057DesiredStateNeverSubstitutesForOriginalNativeAuthority() {
        assertFalse(DisplayActivityState.Unavailable.desired)
        assertTrue(DisplayActivityState.Unsupported.desired)
        assertFalse(DisplayActivityState.Observed(DisplayActivitySignal(1, 0, false)).desired)
        assertTrue(DisplayActivityState.Observed(DisplayActivitySignal(1, 0, true)).desired)
    }

}
