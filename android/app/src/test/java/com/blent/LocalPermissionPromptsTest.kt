package com.blent

import org.junit.Assert.*
import org.junit.Test

class LocalPermissionPromptsTest {
    private class Action : LocalPermissionAction {
        var live = true
        var currentFailure = false
        var completionFailure = false
        val values = mutableListOf<Boolean>()
        override fun current(): Boolean {
            if (currentFailure) throw IllegalStateException("permission owner unavailable")
            return live
        }
        override fun complete(granted: Boolean) {
            values.add(granted)
            if (completionFailure) throw IllegalStateException("callback failed")
        }
    }

    @Test fun t949ReplacementActivityCannotReceiveOldPromptOrLoseItsLauncherToOldDetach() {
        val owner = LocalPermissionPrompts()
        val launched = mutableListOf<LocalPermissionPrompt>()
        val original = owner.attach { launched.add(it); true }
        val oldAction = Action()
        assertTrue(owner.request(oldAction))
        val oldPrompt = launched.single()
        val replacement = owner.attach { launched.add(it); true }
        assertFalse(owner.result(oldPrompt, true))
        assertTrue(oldAction.values.isEmpty())
        assertFalse(owner.detach(original))
        val next = Action()
        assertTrue(owner.request(next)); assertEquals(2, launched.size)
        assertTrue(owner.result(launched.last(), false)); assertEquals(listOf(false), next.values)
        assertFalse(owner.result(launched.last(), true))
        assertTrue(owner.detach(replacement)); assertFalse(owner.request(Action()))
    }

    @Test fun t949OnePendingExactPromptRejectsForgedDuplicateCancelledAndStaleResults() {
        val owner = LocalPermissionPrompts()
        val launched = mutableListOf<LocalPermissionPrompt>()
        val attached = owner.attach { launched.add(it); true }
        val action = Action()
        assertTrue(owner.request(action)); assertFalse(owner.request(Action()))
        val prompt = launched.single()
        assertFalse(owner.result(LocalPermissionPrompt(attached, action), true))
        assertTrue(owner.result(prompt, true)); assertEquals(listOf(true), action.values)
        assertFalse(owner.result(prompt, false))
        assertTrue(owner.request(action)); val cancelled = launched.last()
        owner.cancel(Action()); owner.cancel(action)
        assertFalse(owner.result(cancelled, true))
        assertTrue(owner.request(action)); action.live = false
        assertFalse(owner.result(launched.last(), true)); assertEquals(listOf(true), action.values)
        assertFalse(owner.request(action)); owner.clear(); assertFalse(owner.request(Action()))
    }

    @Test fun t949FailedThrowingAndSynchronousLaunchDoNotRetainOrReplayActions() {
        val owner = LocalPermissionPrompts()
        val action = Action()
        owner.attach { false }; assertFalse(owner.request(action))
        owner.attach { throw IllegalStateException("SDK unavailable") }; assertFalse(owner.request(action))
        owner.attach { assertTrue(owner.result(it, true)); true }
        assertTrue(owner.request(action)); assertTrue(owner.request(action))
        assertEquals(listOf(true, true), action.values)
        action.currentFailure = true; assertFalse(owner.request(action)); action.currentFailure = false
        action.completionFailure = true
        owner.attach { assertFalse(owner.result(it, false)); true }
        assertTrue(owner.request(action)); assertEquals(listOf(true, true, false), action.values)
        owner.clear()
    }
}
