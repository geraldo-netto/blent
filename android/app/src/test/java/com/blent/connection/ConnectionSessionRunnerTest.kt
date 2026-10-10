package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.io.IOException

class ConnectionSessionRunnerTest {
    @Test fun t883BlockedReaderDoesNotStallWriterOrSerializedControl() {
        val rig = ConnectionRunnerRig()
        val frame = rig.frame(); rig.session.output.add(frame)
        rig.runner.advance(0)
        val read = rig.io.read(); val header = rig.io.write()
        assertEquals(40, header.loan.length); rig.io.complete(header, header.loan.length)
        rig.runner.advance(1)
        val body = rig.io.write(); assertSame(frame.bytes, body.loan.bytes)
        rig.io.complete(body, body.loan.length); rig.runner.advance(2)
        val flush = rig.io.flush(); assertTrue(rig.session.written.isEmpty())
        rig.io.complete(flush, 0); rig.runner.advance(3)
        assertSame(frame, rig.session.written.single()); assertSame(read, rig.io.read())
        assertSame(rig.session, rig.runner.endpoints()); assertEquals(3L, rig.session.clock)
        assertNull(rig.runner.failure()); assertFalse(rig.runner.retired())
    }
    private fun receive(rig: ConnectionRunnerRig, bytes: ByteArray, chunk: Int) {
        var offset = 0
        while (offset < bytes.size) {
            val request = rig.io.read()
            val count = minOf(chunk, request.loan.length, bytes.size - offset)
            bytes.copyInto(request.loan.bytes, request.loan.offset, offset, offset + count)
            rig.io.complete(request, count); rig.runner.advance(1)
            offset += count
        }
    }
    @Test fun t883PartialReadsDeliverExactFramesWhileWriterRemainsBlocked() {
        val rig = ConnectionRunnerRig(); val frame = rig.frame(ByteArray(512) { it.toByte() })
        rig.session.output.add(frame); rig.runner.advance(0)
        val write = rig.io.write()
        receive(rig, rig.encoded(frame), 7)
        assertArrayEquals(frame.bytes, rig.session.input.single().bytes)
        assertEquals(frame.header, rig.session.input.single().header)
        assertSame(write, rig.io.write()); assertTrue(rig.session.written.isEmpty())
        assertNull(rig.runner.failure())
    }
    @Test fun t883WriteDeadlineQuarantinesUntilActualNativeRetirement() {
        val rig = ConnectionRunnerRig(); rig.session.output.add(rig.frame()); rig.runner.advance(0)
        rig.runner.advance(10)
        assertNotNull(rig.runner.failure()); assertTrue(rig.io.stopping); assertTrue(rig.session.disconnected)
        assertFalse(rig.runner.retired()); assertFalse(rig.runner.terminationRequired())
        rig.runner.advance(14); assertFalse(rig.runner.terminationRequired())
        rig.runner.advance(15); assertTrue(rig.runner.terminationRequired()); assertFalse(rig.runner.retired())
        rig.io.gone = true; rig.runner.advance(16)
        assertTrue(rig.runner.retired()); assertEquals(1, rig.session.retirementCount)
        rig.runner.advance(17); assertEquals(1, rig.session.retirementCount)
    }
    @Test fun t883ReadDeadlineCoversStalledPartialFrame() {
        val rig = ConnectionRunnerRig(); rig.runner.advance(0)
        val request = rig.io.read(); request.loan.bytes[0] = 66
        rig.io.complete(request, 1); rig.runner.advance(1)
        rig.runner.advance(10); assertNull(rig.runner.failure())
        rig.runner.advance(11); assertNotNull(rig.runner.failure()); assertTrue(rig.io.stopping)
    }
    @Test fun t883CancelledExactCallbacksRetireWithoutDeliveringPartialMedia() {
        val rig = ConnectionRunnerRig(); rig.session.output.add(rig.frame()); rig.runner.advance(0)
        val read = rig.io.read(); val write = rig.io.write()
        rig.runner.stop(1); rig.runner.stop(1)
        rig.io.complete(read, -1, IOException("cancelled")); rig.io.complete(write, -1, IOException("cancelled"))
        rig.runner.advance(2)
        assertTrue(rig.session.input.isEmpty()); assertTrue(rig.session.written.isEmpty())
        assertNull(rig.runner.failure()); assertFalse(rig.runner.retired())
        rig.io.gone = true; rig.runner.advance(3); assertTrue(rig.runner.retired())
    }
    @Test fun t883CloseFailureRequiresProcessTerminationEvenAfterCallsExit() {
        val rig = ConnectionRunnerRig(); val failure = IOException("close failed")
        rig.runner.stop(0); rig.io.gone = true; rig.io.closeError = failure; rig.runner.advance(1)
        assertTrue(rig.runner.retired()); assertTrue(rig.runner.terminationRequired())
        assertSame(failure, rig.runner.failure())
    }
    @Test fun t883InvalidClockAndLimitsRejectBeforeIssuingNativeWork() {
        for (timeout in listOf(Long.MIN_VALUE, -1, 0, 60001, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionRunLimits(ioTimeoutMs = timeout) }
            assertThrows(IllegalArgumentException::class.java) { ConnectionRunLimits(retirementTimeoutMs = timeout) }
        }
        val rig = ConnectionRunnerRig()
        for (now in listOf(Long.MIN_VALUE, -1, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { rig.runner.advance(now) }
        }
        assertTrue(rig.io.requests.isEmpty()); rig.runner.advance(3)
        assertThrows(IllegalArgumentException::class.java) { rig.runner.stop(2) }
        assertFalse(rig.io.stopping); rig.runner.stop(3)
    }
    @Test fun t883CompletionRacingActualRetirementCannotFailCleanStop() {
        val rig = ConnectionRunnerRig()
        val racing = object : ConnectionIo by rig.io {
            override fun retired(): Boolean {
                rig.io.complete(rig.io.read(), -1)
                return true
            }
        }
        val runner = ConnectionSessionRunner(rig.session, racing)
        runner.advance(0); runner.stop(1); runner.advance(2)
        assertTrue(runner.retired()); assertNull(runner.failure())
        runner.advance(3)
        assertNull("Exact completion after actual native exit is no longer protocol work", runner.failure())
        assertEquals(1, rig.session.retirementCount)
    }

}
