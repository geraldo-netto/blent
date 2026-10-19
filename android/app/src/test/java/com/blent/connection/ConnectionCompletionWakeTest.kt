package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionCompletionWakeTest {
    @Test fun t1096OriginalCleanupCompletionWakesWithoutAdvancingPollTimer() {
        val scheduler = ConnectionActorScheduler()
        val scope = CoroutineScope(SupervisorJob() + scheduler)
        val admission = RunAdmission().apply { ready = selection }
        val worker = AdmissionWorker()
        var originalWake: (() -> Unit)? = null
        val run = ConnectionAdmissionRun(ConnectionWorkerSpec("fixture", timerMs = 1000),
            { wake -> originalWake = wake; admission }, { _, _ -> worker }, {}, dispatcher = scheduler)
        val executing = scope.async { run.execute() }
        try {
            scheduler.runCurrent(); assertTrue(worker.executing.isCompleted)
            run.cancel(); scheduler.runCurrent()
            assertEquals(1, worker.retires.get()); assertFalse(executing.isCompleted)
            worker.nativeEnd.complete(Unit); scheduler.runCurrent()
            assertTrue("T1096 actual cleanup completion still waits for poll timer", executing.isCompleted)
            assertEquals(0L, scheduler.now)
            runBlocking { executing.await(); run.retire() }
            repeat(64) { checkNotNull(originalWake)() }
            scheduler.runCurrent(); assertTrue(scheduler.empty())
        } finally {
            worker.nativeEnd.complete(Unit); run.cancel(); scheduler.advanceTo(1000)
            assertTrue("T1096 original execution cleanup did not join", executing.isCompleted)
            runBlocking { executing.await(); run.retire() }
            scope.cancel(); scheduler.runCurrent(); assertTrue(scheduler.empty())
        }
    }
}
