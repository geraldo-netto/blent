package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.io.IOException

class ConnectionRunnerInvalidTest {
    @Test fun t883ForeignAndDuplicateNativeReceiptsFailClosed() {
        val rig = ConnectionRunnerRig(); rig.runner.advance(0)
        rig.io.complete(rig.io.read().copy(), 0); rig.runner.advance(1)
        assertTrue(rig.runner.failure() is IllegalArgumentException); assertTrue(rig.io.stopping)
        assertFalse(rig.runner.retired())
        val writing = ConnectionRunnerRig(); writing.session.output.add(writing.frame()); writing.runner.advance(0)
        val first = writing.io.write(); writing.io.complete(first, first.loan.length); writing.runner.advance(1)
        writing.io.complete(first, first.loan.length); writing.runner.advance(2)
        assertTrue(writing.runner.failure() is IllegalArgumentException)
        assertTrue(writing.session.written.isEmpty())
    }
    @Test fun t883InvalidNativeCountsCannotAdvanceBoundsOrDeliverFrames() {
        for (count in listOf(Int.MIN_VALUE, -1, 41, Int.MAX_VALUE)) {
            val reading = ConnectionRunnerRig(); reading.runner.advance(0)
            reading.io.complete(reading.io.read(), count); reading.runner.advance(1)
            assertNotNull(reading.runner.failure()); assertTrue(reading.session.input.isEmpty())
            val writing = ConnectionRunnerRig(); writing.session.output.add(writing.frame()); writing.runner.advance(0)
            writing.io.complete(writing.io.write(), count); writing.runner.advance(1)
            assertNotNull(writing.runner.failure()); assertTrue(writing.session.written.isEmpty())
        }
    }
    @Test fun t883RejectedNativeAdmissionStopsAndRetainsOwnership() {
        val reading = ConnectionRunnerRig(); reading.io.accepting = false; reading.runner.advance(0)
        assertTrue(reading.io.stopping); assertNotNull(reading.runner.failure()); assertFalse(reading.runner.retired())
        val writing = ConnectionRunnerRig(); writing.runner.advance(0)
        writing.session.output.add(writing.frame()); writing.io.accepting = false; writing.runner.advance(1)
        assertTrue(writing.io.stopping); assertNotNull(writing.runner.failure()); assertFalse(writing.runner.retired())
    }
    @Test fun t883NativeFailuresPreserveFirstErrorAcrossCleanup() {
        for (write in listOf(false, true)) {
            val rig = ConnectionRunnerRig(); rig.session.output.add(rig.frame()); rig.runner.advance(0)
            val failure = IOException("native I/O")
            val request = if (write) rig.io.write() else rig.io.read()
            rig.io.complete(request, 0, failure); rig.runner.advance(1)
            assertSame(failure, rig.runner.failure()); assertTrue(rig.io.stopping)
            rig.io.gone = true; rig.io.closeError = IOException("close"); rig.runner.advance(2)
            assertSame(failure, rig.runner.failure()); assertTrue(rig.runner.terminationRequired())
        }
    }
    private fun flushing(): ConnectionRunnerRig {
        val rig = ConnectionRunnerRig(); rig.session.output.add(rig.frame()); rig.runner.advance(0)
        rig.io.complete(rig.io.write(), 40); rig.runner.advance(1)
        rig.io.complete(rig.io.write(), 3); rig.runner.advance(2)
        return rig
    }
    @Test fun t883FlushCompletionCannotClaimPayloadBytesOrOutliveDeadline() {
        val invalid = flushing(); invalid.io.complete(invalid.io.flush(), 1); invalid.runner.advance(3)
        assertNotNull(invalid.runner.failure()); assertTrue(invalid.session.written.isEmpty())
        val stalled = flushing(); stalled.runner.advance(10)
        assertNotNull(stalled.runner.failure()); assertTrue(stalled.io.stopping)
        assertTrue(stalled.session.written.isEmpty())
    }
    @Test fun t883BoundedZeroProgressAndMalformedHeadersKeepAbsoluteDeadline() {
        val rig = ConnectionRunnerRig(); rig.session.output.add(rig.frame()); rig.runner.advance(0)
        repeat(64) {
            rig.io.complete(rig.io.read(), 0); rig.io.complete(rig.io.write(), 0); rig.runner.advance(1)
        }
        rig.runner.advance(10); assertNotNull(rig.runner.failure()); assertTrue(rig.io.stopping)
        val corrupt = ConnectionRunnerRig(); corrupt.runner.advance(0)
        val request = corrupt.io.read(); request.loan.bytes.fill(127)
        corrupt.io.complete(request, 40); corrupt.runner.advance(1)
        assertNotNull(corrupt.runner.failure()); assertTrue(corrupt.session.input.isEmpty())
    }
}
