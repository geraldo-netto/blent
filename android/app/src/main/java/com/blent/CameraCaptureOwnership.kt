package com.blent

import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import java.util.concurrent.atomic.AtomicBoolean

/** Explicit target takes precedence; retain latest ADB invitation for fallback. */
internal class CameraCaptureSource {
    private var invitation: CameraEndpoint? = null
    var target: CameraCaptureTarget? = null; private set
    val endpoint: CameraEndpoint? get() = target?.endpoint ?: invitation
    fun invitation(value: CameraEndpoint?): Boolean {
        if (invitation == value) return false
        invitation = value
        return target == null
    }
    fun target(value: CameraCaptureTarget?): Boolean {
        if (target === value) return false
        target = value
        return true
    }
}

/** Lazy native acquisition: permission and the exact retained claim are checked
 * before factory/native execution. All created workers remain owned by retire. */
internal class CameraCaptureRun(
    private val claim: CameraCaptureClaim?,
    private val permission: () -> Boolean,
    private val create: () -> LifecycleRun,
) : LifecycleRun {
    private val stopped = AtomicBoolean()
    private val admitted = AtomicBoolean()
    @Volatile private var worker: LifecycleRun? = null
    override suspend fun execute() {
        currentCoroutineContext().ensureActive()
        check(admitted.compareAndSet(false, true)) { "Camera request already executed" }
        check(!stopped.get()) { "Camera request retired" }
        check(permission()) { "Camera permission unavailable" }
        check(claim?.current() != false) { "Camera target no longer current" }
        val run = create()
        worker = run
        currentCoroutineContext().ensureActive()
        check(!stopped.get()) { "Camera request retired" }
        check(permission()) { "Camera permission unavailable" }
        check(claim?.current() != false) { "Camera target no longer current" }
        run.execute()
    }
    override fun cancel() { stopped.set(true); worker?.cancel() }
    override suspend fun retire() { cancel(); worker?.retire() }
}
