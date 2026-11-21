package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

/** T1518: fault after every storage transition, including durable publication
 * and retirement. The map models bytes that survive process death, not power
 * loss; native rename/force/lock behavior remains in AndroidPeerTrustStoreTest. */
class TrustPublicationTest {
    private class Files : TrustFileAccess {
        val records = mutableMapOf<String, ByteArray>()
        var steps = 0
        var failAfter = Int.MAX_VALUE
        private fun step() { check(++steps != failAfter) { "injected interruption" } }
        override fun lock() = Unit
        override fun close() = Unit
        override fun exists(name: String) = records.containsKey(name)
        override fun read(name: String, maximum: Int): ByteArray? = records[name]?.also { require(it.size <= maximum) }?.copyOf()
        override fun writeNew(name: String, bytes: ByteArray) {
            check(!records.containsKey(name)); records[name] = bytes.copyOf(); step()
        }
        override fun replace(source: String, target: String) { records[target] = checkNotNull(records.remove(source)); step() }
        override fun syncDirectory() = step()
        override fun remove(name: String) { checkNotNull(records.remove(name)); step() }
    }
    private fun record(revision: Long, peers: List<ConnectionPeer> = emptyList()) =
        ConnectionTrustCodec.encode(TrustSnapshot(savedPeer(0), revision, peers))
    private fun fixture() = Files().apply { records["trust.bin"] = record(1, listOf(savedPeer(1))) }

    @Test fun t1518EveryPublicationTransitionCanResumeWithoutRestoringApproval() {
        val complete = fixture()
        TrustPublication(complete).replace(record(2))
        for (step in 1..complete.steps) {
            val files = fixture().apply { failAfter = step }
            assertThrows(IllegalStateException::class.java) { TrustPublication(files).replace(record(2)) }
            files.failAfter = Int.MAX_VALUE
            TrustPublication(files).recover()
            assertArrayEquals("step $step", record(2), files.records["trust.bin"])
            assertEquals(setOf("trust.bin"), files.records.keys)
        }
    }

    @Test fun t1518EveryResetTransitionResumesAndLeavesNoApproval() {
        val complete = damaged()
        TrustPublication(complete).reset()
        for (step in 1..complete.steps) {
            val files = damaged().apply { failAfter = step }
            assertThrows(IllegalStateException::class.java) { TrustPublication(files).reset() }
            files.failAfter = Int.MAX_VALUE
            TrustPublication(files).recover()
            assertTrue("step $step", files.records.isEmpty())
            TrustPublication(files).reset()
            assertTrue(files.records.isEmpty())
        }
    }
    private fun damaged() = fixture().apply {
        records["trust.pending"] = byteArrayOf(1)
        records["trust.tmp"] = byteArrayOf(7)
    }

    @Test fun t1518EveryTruncatedIntentFailsClosedUntilExplicitReset() {
        val intended = record(2)
        for (size in 0 until intended.size) {
            val files = fixture().apply { records["trust.pending"] = intended.copyOf(size) }
            assertThrows(Exception::class.java) { TrustPublication(files).recover() }
            assertArrayEquals(record(1, listOf(savedPeer(1))), files.records["trust.bin"])
            assertArrayEquals(intended.copyOf(size), files.records["trust.pending"])
            TrustPublication(files).reset()
            assertTrue(files.records.isEmpty())
        }
    }

    @Test fun t1518InterruptedRecoveryRetainsTheOriginalIntendedRecord() {
        val complete = recovery()
        TrustPublication(complete).recover()
        for (step in 1..complete.steps) {
            val files = recovery().apply { failAfter = step }
            assertThrows(Exception::class.java) { TrustPublication(files).recover() }
            files.failAfter = Int.MAX_VALUE
            TrustPublication(files).recover()
            assertArrayEquals(record(2), files.records["trust.bin"])
            assertEquals(setOf("trust.bin"), files.records.keys)
        }
    }
    private fun recovery() = fixture().apply {
        records["trust.pending"] = record(2)
        records["trust.tmp"] = byteArrayOf(1)
    }

    @Test fun t1518UnknownTemporaryDataIsNeverPublishedWithoutIntent() {
        val files = fixture().apply { records["trust.tmp"] = record(2) }
        assertThrows(IllegalStateException::class.java) { TrustPublication(files).recover() }
        assertArrayEquals(record(1, listOf(savedPeer(1))), files.records["trust.bin"])
        TrustPublication(files).reset()
        assertTrue(files.records.isEmpty())
    }
}
