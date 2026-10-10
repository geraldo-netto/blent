package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong

class ConnectionSetupBoundaryTest {
    @Test fun t886StartedThenThrowingThreadRemainsOwnedUntilActualExit() {
        val backend = SetupBackend().apply { authenticateGate = CountDownLatch(1) }
        val store = SetupStore()
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(setupPeer(backend.local), 1, listOf(setupPeer(backend.remote))))
        val owner = ConnectionSetupOwner(backend, { store }, { 10 }, threads = { name, body ->
            object : Thread(body, name) {
                override fun start() {
                    super.start()
                    if (name.endsWith("authority")) {
                        check(backend.entered.await(2, TimeUnit.SECONDS))
                        throw IllegalStateException("start returned failure after side effect")
                    }
                }
            }.apply { isDaemon = true }
        })
        try {
            setupAwait { backend.closed }; Thread.sleep(20)
            assertFalse("Native authority thread still owns blocked authenticate", owner.retired())
        } finally {
            backend.authenticateGate.countDown(); owner.stop(); setupAwait { store.closed }
        }
    }

    @Test fun t886ClockExhaustionStillClosesAndRetainsNativeOwner() {
        val backend = SetupBackend(); val store = SetupStore(); val clock = AtomicLong(10)
        val owner = ConnectionSetupOwner(backend, { store }, clock::get)
        try {
            setupAwait { owner.poll() is ConnectionWorkerEvent.Pairing }
            clock.set(Long.MAX_VALUE)
            owner.advance()
            setupAwait { owner.retired() }
            assertNotNull(owner.failure())
        } finally { clock.set(10); owner.stop() }
    }
}
