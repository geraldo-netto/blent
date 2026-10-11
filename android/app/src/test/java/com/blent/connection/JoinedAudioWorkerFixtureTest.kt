package com.blent.connection

import android.os.IBinder
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class JoinedAudioWorkerFixtureTest {
    @Test fun t1005IndexedDeathSnapshotCannotRaceRecipientUnlink() {
        val deaths = object : CopyOnWriteArrayList<IBinder.DeathRecipient>() {
            override fun get(index: Int): IBinder.DeathRecipient {
                // Deterministic interleaving: unlink after Kotlin reads size,
                // immediately before its single-element indexed snapshot.
                clear()
                return super.get(index)
            }
        }
        val called = AtomicInteger()
        deaths.add(IBinder.DeathRecipient { called.incrementAndGet() })
        val rig = JoinedAudioWorkerFixture(deaths = deaths)
        try {
            rig.die()
            assertFalse(rig.alive)
            assertEquals(1, called.get())
        } finally { deaths.clear(); rig.close() }
    }

    @Test fun t1005DeathCallbacksCanUnlinkThemselvesWithoutLosingSnapshotRecipients() {
        for (count in listOf(0, 1, 2, 32)) {
            val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
            val called = AtomicInteger()
            repeat(count) {
                lateinit var recipient: IBinder.DeathRecipient
                recipient = IBinder.DeathRecipient { called.incrementAndGet(); deaths.remove(recipient) }
                deaths.add(recipient)
            }
            val rig = JoinedAudioWorkerFixture(deaths = deaths)
            try {
                rig.die()
                assertEquals(count, called.get()); assertTrue(deaths.isEmpty()); assertFalse(rig.alive)
                rig.die(); assertEquals(count, called.get())
            } finally { rig.close() }
        }
    }
}
