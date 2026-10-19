package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

/** T1093: expose existing absolute budgets; no timer or admission policy change. */
class ConnectionTransportDeadlineTest {
    @Test fun t1093ClockReportsOriginalBudgetUntilClearFailureOrCancellation() {
        val clock = ConnectionFrameClock(10)
        assertEquals(ConnectionDeadline.IDLE, clock.nextDeadline())
        clock.start(4); assertEquals(ConnectionDeadline.at(14), clock.nextDeadline())
        clock.check(13); assertEquals(ConnectionDeadline.at(14), clock.nextDeadline())
        assertThrows(IllegalStateException::class.java) { clock.check(14) }
        assertEquals(ConnectionDeadline.IDLE, clock.nextDeadline())
        val cleared = ConnectionFrameClock(1)
        cleared.start(Long.MAX_VALUE - 1)
        assertEquals(ConnectionDeadline.at(Long.MAX_VALUE), cleared.nextDeadline())
        cleared.clear(); assertEquals(ConnectionDeadline.IDLE, cleared.nextDeadline())
        cleared.cancel(); assertEquals(ConnectionDeadline.IDLE, cleared.nextDeadline())
    }

    @Test fun t1093IdleReadPartialHeaderAndCancelledLoanHaveDistinctLifetimes() {
        val reader = ConnectionFrameReader(10)
        val first = checkNotNull(reader.read(0))
        assertEquals(ConnectionDeadline.IDLE, reader.nextDeadline())
        first.bytes[0] = 66; reader.readComplete(first, 1, 7)
        assertEquals(ConnectionDeadline.at(17), reader.nextDeadline())
        val pending = checkNotNull(reader.read(8))
        reader.readComplete(pending, 0, 9)
        assertEquals(ConnectionDeadline.at(17), reader.nextDeadline())
        reader.read(10); reader.cancel()
        assertEquals(ConnectionDeadline.IDLE, reader.nextDeadline())
        assertTrue("T1093 no budget is not an original native read fence", reader.nativePending())
    }

    @Test fun t1093WriterKeepsAbsoluteDeadlineThroughPartialWritesAndFlush() {
        val writer = ConnectionFrameWriter(10)
        val frame = ConnectionRunnerRig().frame()
        assertEquals(ConnectionDeadline.IDLE, writer.nextDeadline())
        writer.begin(frame, 3)
        val first = checkNotNull(writer.write(4)); writer.writeComplete(first, 1, 5)
        assertEquals(ConnectionDeadline.at(13), writer.nextDeadline())
        val header = checkNotNull(writer.write(6)); writer.writeComplete(header, header.length, 7)
        val body = checkNotNull(writer.write(8)); writer.writeComplete(body, body.length, 9)
        val flush = checkNotNull(writer.flush(10))
        assertEquals(ConnectionDeadline.at(13), writer.nextDeadline())
        assertSame(frame, writer.flushComplete(flush, 12))
        assertEquals(ConnectionDeadline.IDLE, writer.nextDeadline())
        writer.begin(frame, 13); writer.cancel()
        assertEquals(ConnectionDeadline.IDLE, writer.nextDeadline())
    }

    @Test fun t1093ResourceBudgetsExcludeAdmittedClosingAndDisconnectedSlots() {
        val ticket = LocalPumpRig().lease.connection
        val resources = ConnectionResources(ConnectionResources.Role.TABLET, ticket)
        assertEquals(ConnectionDeadline.IDLE, resources.nextDeadline())
        val camera = resources.offer(ConnectionManagement.Key(ConnectionManagement.Resource.CAMERA, 1, 1), 1, 5, 20)
        val display = resources.offer(ConnectionManagement.Key(ConnectionManagement.Resource.DISPLAY, 2, 1), 2, 6, 8)
        assertEquals(ConnectionDeadline.at(14), resources.nextDeadline())
        resources.prepared(display, 7); resources.commit(display, 8); resources.admitted(display, 9)
        assertEquals(ConnectionDeadline.at(25), resources.nextDeadline())
        assertTrue(resources.current(display))
        resources.expire(25)
        assertEquals(ConnectionDeadline.IDLE, resources.nextDeadline())
        assertFalse(resources.owns(camera, 25)); assertTrue(resources.current(display))
        resources.disconnect(); assertEquals(ConnectionDeadline.IDLE, resources.nextDeadline())
    }

    @Test fun t1093ResourceDeadlineMaximumAndCancelledEarliestRemainExact() {
        val resources = ConnectionResources(ConnectionResources.Role.TABLET, LocalPumpRig().lease.connection)
        val first = resources.offer(ConnectionManagement.Key(ConnectionManagement.Resource.CAMERA, 1, 1), 1, 0, 1)
        resources.offer(ConnectionManagement.Key(ConnectionManagement.Resource.DISPLAY, 2, 1), 2, Long.MAX_VALUE - 1, 1)
        assertEquals(ConnectionDeadline.at(1), resources.nextDeadline())
        resources.revoke(first)
        assertEquals(ConnectionDeadline.at(Long.MAX_VALUE), resources.nextDeadline())
        resources.expire(Long.MAX_VALUE)
        assertEquals(ConnectionDeadline.IDLE, resources.nextDeadline())
    }

    @Test fun t1093TabletSessionExposesPendingAdmissionWithoutNativeSettlement() {
        val rig = TabletSessionRig(timeout = 10)
        assertEquals(ConnectionDeadline.IDLE, rig.tablet.nextDeadline())
        val lease = rig.offered()
        assertEquals(ConnectionDeadline.at(10), rig.tablet.nextDeadline())
        rig.tablet.tick(10)
        assertEquals(ConnectionDeadline.IDLE, rig.tablet.nextDeadline())
        assertFalse(rig.tablet.current(lease))
        rig.tablet.nativeRetired(lease)
        rig.tablet.disconnect(); rig.tablet.transportRetired()
    }

    @Test fun t1093RunnerPreservesKnownBudgetsAlongsideUnknownProviders() {
        val rig = ConnectionRunnerRig()
        assertEquals(ConnectionDeadline.UNKNOWN, rig.runner.nextDeadline())
        var budget = ConnectionDeadline.at(50)
        val session = object : ConnectionSession by rig.session, ConnectionDeadlineSource {
            override fun nextDeadline(): ConnectionDeadline = budget
        }
        val runner = ConnectionSessionRunner(session, rig.io, ConnectionRunLimits(10, 5))
        assertEquals(ConnectionDeadline.at(50), runner.nextDeadline())
        rig.session.output.add(rig.frame()); runner.advance(0)
        assertEquals(ConnectionDeadline.at(10), runner.nextDeadline())
        budget = ConnectionDeadline.at(2)
        assertEquals(ConnectionDeadline.at(2), runner.nextDeadline())
        budget = ConnectionDeadline.UNKNOWN
        assertEquals(ConnectionDeadline(10, false), runner.nextDeadline())
        runner.stop(1)
        assertEquals(ConnectionDeadline.at(6), runner.nextDeadline()); assertFalse(runner.retired())
        runner.advance(6)
        assertTrue(runner.terminationRequired()); assertFalse(runner.retired())
        assertEquals(ConnectionDeadline.IDLE, runner.nextDeadline())
        rig.io.gone = true; runner.advance(7)
        assertTrue(runner.retired()); assertEquals(ConnectionDeadline.IDLE, runner.nextDeadline())
    }
}
