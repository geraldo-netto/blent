package com.blent.connection

import com.blent.LifecycleRun
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map

internal enum class ConnectionUse { CONNECTION, SAVED_TRUST }

/** One application-owned native access slot, independent of Activity lifetime.
 * Mode observations are metadata; only the original permit can release its slot.
 * Production callers use guard: original private worker/store and completion-relay
 * retirement is its proof premise. Independent native media consumers keep their
 * own owners and original authority; they cannot retarget a replacement worker.
 * Unknown connection cleanup retains the slot.
 * No worker, timer, native handle or observer scope is allocated by this gate. */
internal class ConnectionAccess {
    private val held = MutableStateFlow<Permit?>(null)
    val changes = held.map { it?.use }.distinctUntilChanged()
    fun current(): ConnectionUse? = held.value?.use

    fun guard(use: ConnectionUse, run: LifecycleRun): LifecycleRun = ConnectionAccessRun(this, use, run)

    fun acquire(use: ConnectionUse): Permit {
        val permit = Permit(this, use)
        check(held.compareAndSet(null, permit)) { "USB connection or saved-computer access is active or closing." }
        return permit
    }
    private fun release(permit: Permit) {
        check(held.compareAndSet(permit, null)) { "Foreign connection access release" }
    }

    class Permit internal constructor(private val owner: ConnectionAccess, val use: ConnectionUse) : AutoCloseable {
        private val closed = AtomicBoolean()
        override fun close() {
            if (closed.compareAndSet(false, true)) owner.release(this)
        }
    }
}

/** ResourceLane serializes execute/retire and joins the logical execution first.
 * cancel may race either; the native run must honor cutoff and actual retirement.
 * Construction and unexecuted retirement must be native-inert. Successful retire
 * settles the guarded connection resources. Separately owned native media claims
 * may remain, with cutoff authority bound only to their original worker. */
private class ConnectionAccessRun(
    private val access: ConnectionAccess,
    private val use: ConnectionUse,
    private val run: LifecycleRun,
) : LifecycleRun {
    private val started = AtomicBoolean()
    private val stopped = AtomicBoolean()
    private var permit: ConnectionAccess.Permit? = null
    @Volatile private var retired = false

    override suspend fun execute() {
        check(started.compareAndSet(false, true)) { "Connection access run already started" }
        check(!stopped.get()) { "Connection access run stopped" }
        permit = access.acquire(use)
        currentCoroutineContext().ensureActive()
        check(!stopped.get()) { "Connection access run stopped" }
        run.execute()
    }
    override fun cancel() {
        if (!stopped.compareAndSet(false, true)) return
        if (!retired) run.cancel()
    }
    override suspend fun retire() {
        stopped.set(true)
        if (retired) return
        run.retire()
        retired = true
        permit?.close()
        permit = null
    }
}
