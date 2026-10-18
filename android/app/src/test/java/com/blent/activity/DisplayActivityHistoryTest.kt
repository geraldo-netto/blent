package com.blent.activity

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayActivityHistoryTest {
    private fun greeting(active: Boolean) = JSONObject().put("status", "connected")
        .put("display_activity", JSONObject().put("version", 1).put("revision", 1).put("stopped", 0).put("active", active))
    @Test fun t1067LatestClosedViewPreservesSupportedSourceLossWithoutGrantingWork() {
        for (active in listOf(false, true)) {
            val inbox = DisplayActivityInbox()
            assertFalse(inbox.update.supportKnown)
            inbox.receive(greeting(active))
            val source = inbox.update.source
            inbox.close()
            val latest = inbox.update
            assertSame(source, latest.source)
            assertSame(DisplayActivityState.Unavailable, latest.state)
            assertFalse(latest.state.desired)
            assertTrue("T1067 coalesced closure erased known supported-source loss", latest.supportKnown)
            inbox.close(); assertSame(latest, inbox.update)
        }
    }
    @Test fun t1067MalformedAnnouncedSupportCannotBecomeInitialWaitingFallback() {
        for (invalid in listOf(JSONObject.NULL, "invalid", 0, true, JSONObject().put("version", 2))) {
            val inbox = DisplayActivityInbox()
            assertTrue(runCatching { inbox.receive(JSONObject().put("status", "connected").put("display_activity", invalid)) }.isFailure)
            assertSame(DisplayActivityState.Unavailable, inbox.update.state)
            assertTrue("T1067 malformed announcement erased support knowledge", inbox.update.supportKnown)
            assertFalse(inbox.update.state.desired)
        }
    }
    @Test fun t1067FreshOriginalRetainsLossUntilItsOwnExplicitLegacyGreeting() {
        for (known in listOf(false, true)) {
            val previous = DisplayActivityInbox(known)
            previous.close()
            val fresh = DisplayActivityInbox(previous.update.supportKnown)
            assertNotSame(previous.update.source, fresh.update.source)
            assertEquals(known, fresh.update.supportKnown)
            assertFalse(fresh.update.state.desired)
            fresh.receive(JSONObject().put("status", "connected"))
            assertFalse(fresh.update.supportKnown)
            assertSame(DisplayActivityState.Unsupported, fresh.update.state)
            fresh.close()
            assertFalse(fresh.update.supportKnown)
        }
    }
    @Test fun t1067ClosedLegacySourceCannotAcceptLateSupportClaims() {
        val inbox = DisplayActivityInbox()
        inbox.receive(JSONObject().put("status", "connected")); inbox.close()
        val original = inbox.update
        for (late in listOf(greeting(true), greeting(false), JSONObject().put("display_activity", "malformed"))) {
            assertTrue(runCatching { inbox.receive(late) }.isFailure)
            assertSame(original, inbox.update)
            assertFalse(inbox.update.supportKnown)
        }
    }
    @Test fun t1067ImmutableHistoryCannotContradictObservedOrUnsupportedState() {
        val observed = DisplayActivityState.Observed(DisplayActivitySignal(1, 0, true))
        assertTrue(DisplayActivityUpdate(Any(), observed).supportKnown)
        assertFalse(DisplayActivityUpdate(Any(), DisplayActivityState.Unsupported).supportKnown)
        assertTrue(runCatching { DisplayActivityUpdate(Any(), observed, false) }.isFailure)
        assertTrue(runCatching { DisplayActivityUpdate(Any(), DisplayActivityState.Unsupported, true) }.isFailure)
        for (history in listOf(false, true)) {
            val update = DisplayActivityUpdate(Any(), DisplayActivityState.Unavailable, history)
            assertEquals(history, update.supportKnown); assertFalse(update.state.desired)
        }
    }
}
