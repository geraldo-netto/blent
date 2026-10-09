package com.blent

import java.util.concurrent.atomic.AtomicReference

/** A failed native release keeps its owner reachable and closes admission.
 * Only one run may execute at once; successors must check admission after join. */
internal class RetirementGuard {
    private data class Held(val run: LifecycleRun, val error: Exception)
    private val held = AtomicReference<Held?>(null)
    val error: Exception? get() = held.get()?.error
    fun checkAvailable() { check(error == null) { "Previous native cleanup incomplete: ${error?.message}" } }
    suspend fun retire(run: LifecycleRun): Exception? = try {
        run.retire()
        null
    } catch (error: Exception) {
        held.compareAndSet(null, Held(run, error))
        this.error
    }
}

/** Independent cleanup actions all run; fatal VM errors are not consumed. */
internal fun cleanupAll(vararg actions: () -> Unit): List<Exception> {
    val failures = mutableListOf<Exception>()
    for (action in actions) try { action() } catch (error: Exception) { failures.add(error) }
    return failures
}
