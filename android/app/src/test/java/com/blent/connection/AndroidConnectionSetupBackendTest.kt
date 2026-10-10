package com.blent.connection

import android.app.Application
import android.hardware.usb.UsbManager
import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AndroidConnectionSetupBackendTest {
    private fun opening(opener: (() -> android.os.ParcelFileDescriptor)? = null): AndroidAccessoryOpening {
        val app: Application = RuntimeEnvironment.getApplication()
        val device = nativeAccessory()
        val inventory = AndroidAccessoryInventory(app.getSystemService(UsbManager::class.java), accessoryProfile,
            list = { arrayOf(device) }, permission = { true }, opener = { opener?.invoke() ?: accessoryFile() })
        return AndroidAccessoryOpening(inventory, accessorySelection(device), { true })
    }
    private fun backend(open: () -> AndroidAccessoryOpening, native: SetupBackend = SetupBackend()): AndroidConnectionSetupBackend =
        AndroidConnectionSetupBackend(open, { error("Identity must remain lazy") }, { _, _, _ -> native })

    @Test fun t854ConstructionAndCloseBeforeStartNeverTouchAndroidServices() {
        var calls = 0
        val backend = backend({ calls++; opening() })
        assertEquals(0, calls); assertFalse(backend.retired())
        backend.close(); backend.close(); assertTrue(backend.retired())
        assertThrows(IllegalStateException::class.java) { backend.localIdentity() }
        assertEquals(0, calls)
    }
    @Test fun t854LazyOpeningForwardsSetupAndRetainsNativeLifetime() {
        val native = SetupBackend(); var calls = 0
        val backend = backend({ calls++; opening() }, native)
        assertThrows(IllegalStateException::class.java) { backend.discover(1) }
        try {
            assertSame(native.local, backend.localIdentity()); assertSame(native.local, backend.localIdentity())
            assertEquals(1, calls); assertSame(native.remote, backend.discover(100))
            backend.ready(native.remote, 100); assertEquals(native.proof, backend.authenticate(native.remote))
            assertEquals(4096, backend.negotiate(4096, 100)); assertSame(native.stream, backend.io {})
            assertFalse(backend.retired())
        } finally { backend.close(); accessoryAwait(backend::retired) }
        assertTrue(native.closed)
        assertThrows(IllegalStateException::class.java) { backend.localIdentity() }
    }
    @Test fun t854FailedDefaultIdentityLoadStillOwnsActualSocketAndBridge() {
        val backend = AndroidConnectionSetupBackend({ opening() }, { throw IOException("Keystore unavailable") })
        assertThrows(IOException::class.java) { backend.localIdentity() }
        assertFalse(backend.retired()); backend.close(); accessoryAwait(backend::retired)
    }
    private fun launch(action: () -> Unit): Pair<Thread, AtomicReference<Exception?>> {
        val failure = AtomicReference<Exception?>()
        val thread = Thread { try { action() } catch (error: Exception) { failure.set(error) } }
        thread.start(); return thread to failure
    }
    private fun wait(gate: CountDownLatch) { check(gate.await(3, TimeUnit.SECONDS)) }
    private fun join(thread: Thread) { thread.join(3000); assertFalse(thread.isAlive) }
    @Test fun t854LateOpeningSupplierAfterCloseStaysOwnedUntilActualReturn() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val backend = backend({ entered.countDown(); wait(release); opening() })
        val (worker, failure) = launch { backend.localIdentity() }
        try {
            wait(entered); backend.close(); assertFalse(backend.retired())
        } finally { release.countDown(); join(worker) }
        assertTrue(failure.get() is IllegalStateException); assertTrue(backend.retired())
    }
    @Test fun t854BlockedNativeOpenAllowsCloseWithoutFalseRetirement() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val backend = backend({ opening { entered.countDown(); wait(release); accessoryFile() } })
        val (worker, failure) = launch { backend.localIdentity() }
        try {
            wait(entered); backend.close(); assertFalse(backend.retired())
        } finally { release.countDown(); join(worker) }
        assertTrue(failure.get() is IllegalStateException); accessoryAwait(backend::retired)
    }
    @Test fun t854LateSocketAdapterMustCloseBeforeRetirement() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1); val native = SetupBackend()
        val backend = AndroidConnectionSetupBackend({ opening() }, { error("unused") }, { _, _, _ ->
            entered.countDown(); wait(release); native
        })
        val (worker, failure) = launch { backend.localIdentity() }
        try {
            wait(entered); backend.close(); assertFalse(backend.retired())
        } finally { release.countDown(); join(worker) }
        assertTrue(failure.get() is IllegalStateException); assertTrue(native.closed); accessoryAwait(backend::retired)
    }
    @Test fun t854ThrowingSupplierAndAdapterFactoryQuarantineUncertainOwnership() {
        val missing = backend({ throw IOException("Opening factory failed") })
        assertThrows(IOException::class.java) { missing.localIdentity() }; missing.close(); assertFalse(missing.retired())
        val backend = AndroidConnectionSetupBackend({ opening() }, { error("unused") }, { _, _, _ -> throw IOException("Adapter construction failed") })
        assertThrows(IOException::class.java) { backend.localIdentity() }; backend.close(); assertFalse(backend.retired())
    }
    @Test fun t854LateAdapterCloseFailureCannotBecomeSuccessfulRetirement() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val native = SetupBackend().apply { closeFailure = IOException("Close failed") }
        val backend = AndroidConnectionSetupBackend({ opening() }, { error("unused") }, { _, _, _ ->
            entered.countDown(); wait(release); native
        })
        val (worker, failure) = launch { backend.localIdentity() }
        try { wait(entered); backend.close() }
        finally { release.countDown(); join(worker) }
        assertTrue(failure.get() is IOException); assertFalse(backend.retired())
    }
    @Test fun t854DelegateCloseFailureStillAttemptsAccessoryCleanup() {
        val native = SetupBackend().apply { closeFailure = IOException("TLS close failed") }
        val opening = opening(); val backend = backend({ opening }, native)
        backend.localIdentity()
        assertThrows(IOException::class.java) { backend.close() }
        accessoryAwait(opening::retired); assertFalse(backend.retired())
    }
}
