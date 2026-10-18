package com.blent.activity

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayActivitySourceTest {
    private fun greeting() = JSONObject().put("status", "connected").put("display_activity",
        JSONObject().put("version", 1).put("revision", 1).put("stopped", 0).put("active", true))
    @Test fun t1068ReadOnlyRelevanceRequiresExactCurrentSnapshotAcrossCloseAndReplacement() {
        val source = DisplayActivitySource()
        val view: DisplayActivityView = source
        val initial = view.update
        assertTrue(view.current(initial))
        source.receive(greeting())
        val active = view.update
        assertFalse(view.current(initial)); assertTrue(view.current(active))
        source.close()
        val closed = view.update
        assertFalse(view.current(active)); assertTrue(view.current(closed))
        assertFalse(closed.state.desired); assertTrue(closed.supportKnown)
        source.replace()
        assertFalse(view.current(closed))
        assertNotSame(closed.source, view.update.source)
        assertTrue(view.update.supportKnown)
        assertFalse(view.update.state.desired)
    }
    @Test fun t1068ForeignEquivalentAndDuplicateViewsCannotForgeRelevance() {
        val source = DisplayActivitySource()
        val legacy = JSONObject().put("status", "connected")
        source.receive(legacy)
        val current = source.update
        assertFalse(source.current(DisplayActivitySource().update))
        assertFalse(source.current(DisplayActivityUpdate(current.source, current.state, current.supportKnown)))
        assertFalse(source.receive(legacy))
        assertSame(current, source.update); assertTrue(source.current(current))
        source.close(); source.close()
        assertFalse(source.current(current))
    }
    @Test fun t1068BoundedOriginalHistoryNeverRevivesRetainedOldSnapshots() {
        val source = DisplayActivitySource()
        val retained = ArrayList<DisplayActivityUpdate>()
        repeat(64) {
            val previous = source.update
            retained.add(previous)
            source.receive(greeting())
            retained.add(source.update)
            source.close(); retained.add(source.update)
            source.replace()
            assertTrue(source.update.supportKnown)
            assertTrue(retained.none(source::current))
            assertFalse(source.update.state.desired)
            assertNotSame(previous.source, source.update.source)
        }
        source.receive(JSONObject().put("status", "connected"))
        assertFalse(source.update.supportKnown)
        assertTrue(retained.none(source::current))
    }
}
