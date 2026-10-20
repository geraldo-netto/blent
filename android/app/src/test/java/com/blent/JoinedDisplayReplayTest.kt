package com.blent

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.JUnitCore

/** T1172: original native display assertions must survive a second fixture
 * lifetime; logical completion cannot leave retired codec history admitted. */
class JoinedDisplayReplayTest {
    @Test fun t1172RetiredOriginalDisplayFixtureCanBeCreatedAgain() {
        val runner = JUnitCore()
        repeat(2) { iteration ->
            val result = runner.run(JoinedDisplayActivityTest::class.java)
            println("T1172 settled display lifetime $iteration: ${result.runCount} cases")
            result.failures.forEach { System.err.println(it.trace) }
            assertEquals("T1172 must execute both original SDK oracles", 2, result.runCount)
            assertTrue("T1172 lifetime $iteration: ${result.failures}", result.wasSuccessful())
        }
    }
}
