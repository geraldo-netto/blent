package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionOwnerTest {
    @Test fun t893ApplicationOwnerIsLazyAndNativeNoAccessoryFailureRetires() = runBlocking {
        val previous = ReflectionHelpers.getStaticField<ConnectionBinding?>(ConnectionOwner::class.java, "binding")
        ReflectionHelpers.setStaticField(ConnectionOwner::class.java, "binding", null)
        val app = RuntimeEnvironment.getApplication()
        val owner = ConnectionOwner.get(android.content.ContextWrapper(app))
        try {
            assertSame(owner, ConnectionOwner.get(app))
            assertFalse(owner.state.value.active)
            owner.connect(ConnectionWorkerSpec("usb", timerMs = 1))
            withTimeout(5000) { while (owner.state.value.active || owner.state.value.cleanup != null) delay(1) }
            assertNull(owner.state.value.peer)
            assertFalse(owner.state.value.authenticated)
        } finally {
            owner.disconnect()
            ReflectionHelpers.setStaticField(ConnectionOwner::class.java, "binding", previous)
        }
    }
}
