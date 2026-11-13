package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class DisplayWorkerOwnerTest {
    @Test fun t1416CancelledSponsorSettlesUnopenedRequest() {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val owner = DisplayWorkerOwner(scope)
        val client = owner.open()
        var ended = 0
        scope.cancel()
        owner.replace(client, { error("T1416 cancelled sponsor admitted native work") }, {}, { ended++ }, {})
        assertEquals(1, ended)
        owner.clear(client)
    }

    @Test fun t1416OriginalRetirementSurvivesSponsorCancellation() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val owner = DisplayWorkerOwner(scope)
        val release = CompletableDeferred<Unit>()
        val retiring = CompletableDeferred<Unit>()
        var settlements = 0
        val first = owner.open()
        owner.replace(first, { object : LifecycleRun {
            override suspend fun execute() = awaitCancellation()
            override fun cancel() {}
            override suspend fun retire() { retiring.complete(Unit); release.await(); settlements++ }
        } }, {}, {}, {})
        val next = owner.open()
        retiring.await()
        var ended = 0
        owner.replace(next, { error("T1416 cancelled sponsor replaced the original") }, {}, { ended++ }, {})
        scope.cancel()
        assertEquals(0, settlements)
        release.complete(Unit)
        scope.coroutineContext[Job]!!.join()
        assertEquals(1, settlements)
        assertEquals(1, ended)
        owner.clear(next)
    }

    @Test fun t1416UnknownNativeRetirementFencesEveryClientAndDropsObservers() {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val owner = DisplayWorkerOwner(scope)
        var stale: ((WorkerEvent) -> Unit)? = null
        var events = 0
        var ended = 0
        val status = mutableListOf<String>()
        val first = owner.open()
        try {
            owner.replace(first, { report -> stale = report; failingNative() }, { events++ }, { ended++ }, status::add)
            assertEquals(1, ended)
            assertTrue(status.isNotEmpty())
            assertTrue(status.all { it.contains("unconfirmed") })
            val next = owner.open()
            stale!!.invoke(WorkerEvent.Status("obsolete"))
            assertEquals(0, events)
            owner.clear(first)
            assertTrue(owner.owns(next))
            assertThrows(IllegalStateException::class.java) {
                owner.replace(next, { error("T1416 failed retirement allowed replacement") }, {}, {}, {})
            }
            owner.clear(next)
        } finally { scope.cancel() }
    }

    private fun failingNative() = object : LifecycleRun {
        override suspend fun execute() {}
        override fun cancel() {}
        override suspend fun retire() { error("unconfirmed") }
    }

    @Test fun t1416FactoryFailureSettlesOnceAndAllowsFreshAdmission() {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val owner = DisplayWorkerOwner(scope)
        val failures = mutableListOf<Exception?>()
        val first = owner.open()
        try {
            owner.replace(first, { error("factory") }, {}, failures::add, {})
            assertEquals(listOf("factory"), failures.map { it?.message })
            val next = owner.open()
            owner.replace(first, { error("obsolete") }, {}, {}, {})
            assertFalse(owner.owns(first))
            assertFalse(owner.owns(null))
            owner.replace(next, { object : LifecycleRun {
                override suspend fun execute() {}
                override fun cancel() {}
                override suspend fun retire() {}
            } }, {}, failures::add, {})
            assertEquals(2, failures.size)
            assertNull(failures.last())
            owner.clear(next)
        } finally { scope.cancel() }
    }
}
