package com.blent.connection

import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ClosedReceiveChannelException
import org.junit.Assert.*
import org.junit.Test

class ConnectionWakeTest {
    @Test fun t1768DueWaitPreservesHintAndDoesNotObserveClosedChannel() = runBlocking {
        val wake = Channel<Unit>(Channel.CONFLATED)
        try {
            wake.send(Unit)
            awaitConnectionWake(wake, 0)
            assertEquals(Unit, wake.tryReceive().getOrNull())
            wake.close()
            awaitConnectionWake(wake, 0)
        } finally { wake.cancel() }
    }
    @Test fun t1768ReadyHintSettlesExactlyOneWait() = runBlocking {
        val wake = Channel<Unit>(Channel.CONFLATED)
        try {
            wake.send(Unit)
            withTimeout(3000) { awaitConnectionWake(wake, 1000) }
            assertTrue(wake.tryReceive().isFailure)
            wake.send(Unit)
            assertEquals(Unit, wake.tryReceive().getOrNull())
        } finally { wake.cancel() }
    }
    @Test fun t1768LateHintWakesTheOriginalSuspendedWait() = runBlocking {
        val wake = Channel<Unit>(Channel.CONFLATED)
        try {
            val waiter = async(start = CoroutineStart.UNDISPATCHED) { awaitConnectionWake(wake, 1000) }
            assertFalse(waiter.isCompleted)
            wake.send(Unit)
            withTimeout(3000) { waiter.await() }
            wake.send(Unit)
            assertEquals(Unit, wake.tryReceive().getOrNull())
        } finally { wake.cancel() }
    }
    @Test fun t1768TimeoutRetiresItsReceiverBeforeTheNextHint() = runBlocking {
        val wake = Channel<Unit>(Channel.CONFLATED)
        try {
            withTimeout(3000) { awaitConnectionWake(wake, 1) }
            wake.send(Unit)
            assertEquals(Unit, wake.tryReceive().getOrNull())
        } finally { wake.cancel() }
    }
    @Test fun t1768CancellationRetiresItsReceiverBeforeTheNextHint() = runBlocking {
        val wake = Channel<Unit>(Channel.CONFLATED)
        try {
            repeat(64) {
                val waiter = launch(start = CoroutineStart.UNDISPATCHED) { awaitConnectionWake(wake, 1000) }
                waiter.cancelAndJoin()
                assertTrue(waiter.isCancelled)
                wake.send(Unit)
                assertEquals(Unit, wake.tryReceive().getOrNull())
            }
        } finally { wake.cancel() }
    }
    @Test fun t1768ClosedChannelAndInvalidDelaysRemainExplicitFailures() = runBlocking {
        val wake = Channel<Unit>(Channel.CONFLATED)
        try {
            wake.send(Unit)
            for (delay in listOf(Long.MIN_VALUE, -1L, 1001L, Long.MAX_VALUE)) {
                val failure = runCatching { awaitConnectionWake(wake, delay) }.exceptionOrNull()
                assertTrue(failure is IllegalArgumentException)
            }
            assertEquals(Unit, wake.tryReceive().getOrNull())
            wake.close()
            val failure = runCatching { awaitConnectionWake(wake, 1) }.exceptionOrNull()
            assertTrue(failure is ClosedReceiveChannelException)
        } finally { wake.cancel() }
    }
}
