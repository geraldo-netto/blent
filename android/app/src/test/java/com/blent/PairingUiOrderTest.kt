package com.blent

import com.blent.connection.ConnectionControlsTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.Description
import org.junit.runner.JUnitCore
import org.junit.runner.Request
import org.junit.runner.manipulation.Filter

/** T1076: real Activity teardown must not poison a later pairing UI sandbox. */
class PairingUiOrderTest {
    @Test fun t1076OrientationThenPairingUsesSettledDispatcherApi27() = ordered("[27]")
    @Test fun t1076OrientationThenPairingUsesSettledDispatcherApi34() = ordered("")

    private fun ordered(sdk: String) {
        val runner = JUnitCore()
        for ((type, method) in listOf(
            OrientationTest::class.java to "t238_naturallyLandscapePanelsFollowBothFlips",
            ConnectionControlsTest::class.java to "t893DismissingPairingNeverApproves",
        )) {
            val request = Request.aClass(type).filterWith(object : Filter() {
                override fun describe() = "T1076 $method$sdk"
                override fun shouldRun(description: Description): Boolean =
                    description.methodName == null || description.methodName == method+sdk
            })
            val result = runner.run(request)
            assertTrue("T1076 ${type.simpleName}: ${result.failures}", result.wasSuccessful())
            assertEquals("T1076 must execute original regression", 1, result.runCount)
        }
    }
}
