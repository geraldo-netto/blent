package com.blent

import com.blent.connection.ConnectionActivityTest
import com.blent.connection.ConnectionActivityUiTest
import com.blent.connection.ConnectionControlsTest
import org.junit.Assert.assertTrue
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.Description
import org.junit.runner.JUnitCore
import org.junit.runner.Request
import org.junit.runner.Result
import org.junit.runner.manipulation.Filter

/** T1171: run actual Activity/UI teardown before the original pairing oracle.
 * No framework fields, idling deadlines or original assertions are changed. */
class SavedUiOrderTest {
    @Test fun t1171ActivityUiPreludeThenPairingApi27() = ordered("[27]")
    @Test fun t1171ActivityUiPreludeThenPairingApi34() = ordered("")

    private fun verifyPairing(runner: JUnitCore, sdk: String) {
        val result = runner.run(Request.aClass(ConnectionControlsTest::class.java)
            .filterWith(filter(sdk, "t893DismissingPairingNeverApproves")))
        settled("original pairing" + sdk, result)
        assertTrue("T1171 original pairing oracle: ${result.failures}", result.wasSuccessful())
        assertEquals("T1171 must execute original pairing regression", 1, result.runCount)
    }

    private fun ordered(sdk: String) {
        val runner = JUnitCore()
        val prelude = listOf(ActivityUiContractTest::class.java, AudioActivityAttachmentTest::class.java,
            CameraConsentMainActivityTest::class.java, DisplaySettingsTest::class.java, OrientationTest::class.java,
            ConnectionActivityTest::class.java, ConnectionActivityUiTest::class.java)
        for (type in prelude) {
            val result = runner.run(Request.aClass(type).filterWith(filter(sdk)))
            settled(type.simpleName + sdk, result)
            assertTrue("T1171 ${type.simpleName}: ${result.failures}", result.wasSuccessful())
            assertTrue("T1171 prelude was not executed: ${type.simpleName}", result.runCount > 0)
        }
        verifyPairing(runner, sdk)
    }
    private fun settled(name: String, result: Result) {
        println("T1171 settled $name: ${result.runCount} cases, ${result.failureCount} failures")
        result.failures.forEach { System.err.println(it.trace) }
    }
    private fun filter(sdk: String, method: String? = null): Filter = object : Filter() {
        override fun describe() = "T1171 ${method ?: "Activity/UI prelude"}$sdk"
        override fun shouldRun(description: Description): Boolean {
            val name = description.methodName ?: return true
            if (method != null) return name == method + sdk
            return if (sdk.isEmpty()) !name.contains('[') else name.endsWith(sdk)
        }
    }
}
