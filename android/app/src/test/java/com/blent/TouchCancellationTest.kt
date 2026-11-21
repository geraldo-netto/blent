package com.blent

import android.view.MotionEvent
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class TouchCancellationTest {
    private class Fixture(direct: Boolean = false) {
        val messages = mutableListOf<JSONObject>()
        val motion = MotionTranslator { value, _ -> messages.add(value) }
        init {
            if (direct) motion.setDirectInput(JSONObject().put("protocol", 1).put("mode", "touch")
                .put("touch", true).put("mouse", true).put("negotiated", true))
        }
        fun send(action: Int, vararg pointers: Pair<Int, Int>): List<JSONObject> {
            messages.clear()
            val properties = pointers.map { (id, tool) -> MotionEvent.PointerProperties().apply {
                this.id = id; toolType = tool
            } }.toTypedArray()
            val coordinates = pointers.map { MotionEvent.PointerCoords().apply {
                x = 30f; y = 40f; pressure = 0.5f
            } }.toTypedArray()
            val event = MotionEvent.obtain(0, 10, action, pointers.size, properties, coordinates,
                0, 0, 1f, 1f, 0, 0, 0, 0)
            try { motion.handleMotionEvent(event, 100, 100) } finally { event.recycle() }
            return messages.toList()
        }
    }

    @Test fun t1499_systemCancelRetiresEverySlotWithoutCompletingTouches() {
        for (direct in listOf(false, true)) {
            val fixture = Fixture(direct)
            val finger = MotionEvent.TOOL_TYPE_FINGER
            fixture.send(MotionEvent.ACTION_DOWN, 10 to finger)
            fixture.send(MotionEvent.ACTION_POINTER_DOWN or (1 shl 8), 10 to finger, 31 to finger)
            val cancelled = fixture.send(MotionEvent.ACTION_CANCEL, 31 to finger, 10 to finger)
            assertEquals(setOf(0, 1), cancelled.map { it.getInt("slot") }.toSet())
            assertEquals(listOf(3, 3), cancelled.map { it.getInt("action") })
            assertTrue(cancelled.all { it.getDouble("pressure") == 0.0 })
            assertTrue(fixture.send(MotionEvent.ACTION_MOVE, 10 to finger).isEmpty())
            assertTrue(fixture.send(MotionEvent.ACTION_CANCEL, 10 to finger).isEmpty())
            assertEquals(0, fixture.send(MotionEvent.ACTION_DOWN, 10 to finger).single().getInt("slot"))
        }
    }

    @Test fun t1499_palmReclassificationCancelsOnlyTheRejectedContact() {
        val fixture = Fixture()
        val finger = MotionEvent.TOOL_TYPE_FINGER
        fixture.send(MotionEvent.ACTION_DOWN, 10 to finger)
        fixture.send(MotionEvent.ACTION_POINTER_DOWN or (1 shl 8), 10 to finger, 31 to finger)
        val messages = fixture.send(MotionEvent.ACTION_MOVE, 10 to 5, 31 to finger)
        assertEquals(listOf(3, 2), messages.map { it.getInt("action") })
        assertEquals(listOf(0, 1), messages.map { it.getInt("slot") })
        assertEquals(0.0, messages.first().getDouble("pressure"), 0.0)
        val up = fixture.send(MotionEvent.ACTION_UP, 31 to finger).single()
        assertEquals(1, up.getInt("action"))
        assertEquals(1, up.getInt("slot"))
    }

    @Test fun t1499_newGestureCancelsUnfinishedOriginalBeforeReusingItsSlot() {
        val fixture = Fixture()
        val finger = MotionEvent.TOOL_TYPE_FINGER
        fixture.send(MotionEvent.ACTION_DOWN, 10 to finger)
        val replacement = fixture.send(MotionEvent.ACTION_DOWN, 31 to finger)
        assertEquals(listOf(3, 0), replacement.map { it.getInt("action") })
        assertEquals(listOf(0, 0), replacement.map { it.getInt("slot") })
    }

    @Test fun t1499_normalReleaseStillCompletesOneTouch() {
        val fixture = Fixture()
        val finger = MotionEvent.TOOL_TYPE_FINGER
        fixture.send(MotionEvent.ACTION_DOWN, 10 to finger)
        assertEquals(1, fixture.send(MotionEvent.ACTION_UP, 10 to finger).single().getInt("action"))
        assertTrue(fixture.send(MotionEvent.ACTION_UP, 10 to finger).isEmpty())
    }
}
