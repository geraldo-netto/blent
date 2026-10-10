package com.blent.connection

import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AccessoryUsbShadow::class])
class AccessoryFixtureIsolationTest {
    @Test fun t891NativeFailureFixtureMustNotPoisonLaterPermissionTests() {
        AccessoryUsbShadow.resetRequests()
        try {
            AndroidAccessorySelectionTest().t887NativeRequestFailureCancelsOwnedPendingIntent()
            assertNull("T891: native failure injection escaped its test", AccessoryUsbShadow.failure)
        } finally { AccessoryUsbShadow.resetRequests() }
    }
}
