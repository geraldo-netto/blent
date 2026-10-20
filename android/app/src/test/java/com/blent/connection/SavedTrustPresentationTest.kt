package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class SavedTrustPresentationTest {
    private fun ready(presentation: SavedTrustPresentation, viewer: Any): SavedComputers {
        val token = checkNotNull(presentation.begin(viewer))
        val snapshot = SavedComputers(savedPeer(0), 1, listOf(savedPeer(1)))
        presentation.ended(token, null); presentation.retired(token, SavedTrustResult(snapshot))
        return snapshot
    }
    @Test fun t1168OnlyCurrentViewerAndExactDisplayedSnapshotCanSelect() {
        val p = SavedTrustPresentation(); val first = p.attach()
        assertNull(p.choice(first, savedPeer(1)))
        val snapshot = ready(p, first); val choice = checkNotNull(p.choice(first, savedPeer(1)))
        assertEquals(snapshot.select(savedPeer(1)), p.selection(first, choice))
        assertNull(p.choice(first, savedPeer(2)))
        val second = p.attach(); p.detach(first)
        assertSame(second, p.state.value.viewer)
        assertNull(p.choice(first, savedPeer(1))); assertNull(p.selection(second, choice))
        assertFalse(p.canRequest(first)); assertNull(p.begin(first)); assertFalse(p.stop(first))
        val current = checkNotNull(p.choice(second, savedPeer(1)))
        ready(p, second)
        assertNull(p.selection(second, current))
        p.detach(second); assertNull(p.state.value.viewer)
    }
    @Test fun t1168ForgedSelectionCannotEscapeDisplayedSnapshot() {
        val p = SavedTrustPresentation(); val viewer = p.attach(); val snapshot = ready(p, viewer)
        val wrongPeer = SavedTrustChoice(viewer, snapshot, SavedComputerSelection(savedPeer(0), 1, savedPeer(2)))
        assertNull(p.selection(viewer, wrongPeer))
        for (selected in listOf(SavedComputerSelection(savedPeer(9), 1, savedPeer(1)),
            SavedComputerSelection(savedPeer(0), 2, savedPeer(1)))) {
            assertNull(p.selection(viewer, SavedTrustChoice(viewer, snapshot, selected)))
        }
    }
    @Test fun t1168ResultsAndCallbacksAreBoundToOriginalRequestAndNativeRetirement() {
        val p = SavedTrustPresentation(); val viewer = p.attach(); val token = checkNotNull(p.begin(viewer))
        var admitted = 0
        p.consume(Any()) { error("foreign callback") }; p.consume(token) { admitted++ }; assertEquals(1, admitted)
        assertFalse(p.canRequest(viewer)); assertNull(p.begin(viewer))
        p.ended(Any(), Exception("stale")); assertEquals(SavedTrustPhase.BUSY, p.state.value.phase)
        p.ended(token, null); p.consume(token) { error("late callback") }
        assertEquals(SavedTrustPhase.RETIRING, p.state.value.phase)
        p.cleanup(Any(), "stale", true, true); p.retired(Any(), SavedTrustResult(null))
        p.cleanup(token, "still closing", false, true); assertEquals("still closing", p.state.value.cleanup)
        p.retired(token, SavedTrustResult(null)); assertEquals(SavedTrustPhase.READY, p.state.value.phase)
        p.cleanup(token, "old completion", false, false); assertNull(p.state.value.cleanup)
        assertNull(p.choice(viewer, savedPeer(1)))
    }
    @Test fun t1168CancellationErrorsMissingResultAndUnknownCleanupStayDistinct() {
        val p = SavedTrustPresentation(); val viewer = p.attach(); var token = checkNotNull(p.begin(viewer))
        assertTrue(p.stop(viewer)); assertFalse(p.stop(viewer)); p.ended(token, null)
        p.retired(token, SavedTrustResult(null)); assertEquals(SavedTrustPhase.FAILED, p.state.value.phase)
        assertTrue(p.state.value.failure!!.contains("may have completed"))
        token = checkNotNull(p.begin(viewer)); p.ended(token, IllegalArgumentException("corrupt")); p.retired(token, null)
        assertEquals("corrupt", p.state.value.failure)
        token = checkNotNull(p.begin(viewer)); p.ended(token, Exception()); p.retired(token, null)
        assertEquals("Exception", p.state.value.failure)
        token = checkNotNull(p.begin(viewer)); p.ended(token, null); p.retired(token, null)
        assertTrue(p.state.value.failure!!.contains("No complete"))
        token = checkNotNull(p.begin(viewer)); p.cleanup(token, "not a stall", false, true)
        assertNull(p.state.value.cleanup)
        p.ended(token, null); p.cleanup(token, "native unknown", true, true)
        assertEquals(SavedTrustPhase.QUARANTINED, p.state.value.phase); assertFalse(p.canRequest(viewer))
        assertEquals("native unknown", p.state.value.cleanup)
        assertNull(p.selection(viewer, SavedTrustChoice(viewer,
            SavedComputers(savedPeer(0), 1, listOf(savedPeer(1))), SavedComputerSelection(savedPeer(0), 1, savedPeer(1)))))
    }
}
