package com.blent

import android.content.Context
import android.hardware.camera2.*
import android.os.Looper
import java.util.concurrent.*
import kotlin.coroutines.intrinsics.suspendCoroutineUninterceptedOrReturn
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.*
import org.robolectric.shadow.api.Shadow

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [CameraManagerCaptureShadow::class, CameraDeviceCaptureShadow::class])
@LooperMode(LooperMode.Mode.PAUSED)
class CameraDeviceRetirementTest {
    private suspend fun open(capture: CameraCapture, resources: CameraResources): CameraDevice =
        suspendCoroutineUninterceptedOrReturn { continuation ->
            val method = CameraCapture::class.java.getDeclaredMethod("open", String::class.java,
                CameraResources::class.java, kotlin.coroutines.Continuation::class.java)
            method.isAccessible = true
            method.invoke(capture, "test", resources, continuation)
        }

    private fun opened(future: Future<CameraDevice>): CameraDevice {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
        while (!future.isDone && System.nanoTime() < deadline) {
            Shadows.shadowOf(Looper.getMainLooper()).idle()
            Thread.sleep(1)
        }
        return future.get(1, TimeUnit.SECONDS)
    }

    @Test fun t1462_synchronousOpenFailureReleasesStartupWithoutWaitingForClosure() = runBlocking {
        val resources = CameraResources()
        val failure = runCatching {
            kotlinx.coroutines.suspendCancellableCoroutine<CameraDevice> { continuation ->
                CameraDeviceStartup(resources, continuation).abort(IllegalStateException("open failed"))
            }
        }.exceptionOrNull()
        assertEquals("open failed", failure?.message)
        resources.close()
    }

    @Test fun t1462_deviceRetirementWaitsForOriginalOnClosed() {
        CameraManagerCaptureShadow.failure = 0
        CameraManagerCaptureShadow.beforeOpen = {}
        val resources = CameraResources()
        val executor = Executors.newSingleThreadExecutor()
        val capture = CameraCapture(RuntimeEnvironment.getApplication())
        val camera = opened(executor.submit<CameraDevice> { runBlocking { open(capture, resources) } })
        val native = Shadow.extract<CameraDeviceCaptureShadow>(camera)
        native.deferClosed = true
        val retirement = executor.submit { resources.close() }
        try {
            assertTrue(native.closeRequested.await(5, TimeUnit.SECONDS))
            val premature = try { retirement.get(350, TimeUnit.MILLISECONDS); true }
                catch (_: TimeoutException) { false }
            native.onClosed()
            retirement.get(5, TimeUnit.SECONDS)
            assertFalse("T1462 closure return is not the original onClosed fence", premature)
        } finally {
            native.onClosed()
            retirement.get(5, TimeUnit.SECONDS)
            executor.shutdown()
        }
    }
}
