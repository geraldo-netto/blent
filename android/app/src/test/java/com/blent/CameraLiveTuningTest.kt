package com.blent

import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraLiveTuningTest {
    private fun command(revision: Long = 1) = CameraTuningCommand(revision, CameraSensorTuning(), 3000, 1000, true)
    @Test fun t1468_latestRevisionDeduplicatesNativeEffectsAndRejectsUnsupportedBeforeEffects() {
        var sensors=0; var encodings=0
        val live=CameraLiveTuning(CameraTuningFixture.capabilities,{ sensors++ },{ encodings++ })
        live.accept(command()); live.accept(command()); live.accept(command(2))
        assertEquals(1,sensors);assertEquals(1,encodings);assertEquals(2L,live.revision)
        live.accept(command(3).copy(sensor=CameraSensorTuning(metering=1)))
        assertEquals(1,live.status);assertEquals(1,sensors);assertEquals(1,encodings)
        live.accept(command(4).copy(bitrate=2500));assertEquals(0,live.status);assertEquals(2,encodings)
        live.accept(command(5).copy(floor=500,adaptive=false));assertEquals(3,encodings)
        live.accept(command(4));assertEquals(5L,live.revision)
    }
    @Test fun t1468_nativeFailureNeverReportsSuccessAndReplacementHasIndependentState() {
        val live=CameraLiveTuning(CameraTuningFixture.capabilities,{ throw IllegalStateException("native failed") },{ fail("encoding after failed sensor") })
        live.accept(command()); assertEquals(2,live.status);assertEquals(1L,live.revision)
        val next=CameraLiveTuning(CameraTuningFixture.capabilities,{}, {})
        assertEquals(0L,next.revision);next.accept(command());assertEquals(0,next.status)
        val output=okio.Buffer();val link=CameraLink(output,CameraTuningFixture.ack(1))
        link.receipt(live);link.send(java.nio.ByteBuffer.wrap(byteArrayOf(7)),0,1,0)
        assertEquals(1L,output.readLong());assertEquals(2,output.readByte().toInt())
        assertEquals(command(),link.takeCommand());assertNull(link.takeCommand())
    }
    @Test fun t1468_malformedControlRetiresTransportAndCannotBeReused() {
        for (index in listOf(0,8,10,11,12,13,15,17)) {
            var closed=0
            val bytes=CameraTuningFixture.command.clone();bytes[index]=127
            val link=CameraLink(okio.Buffer(),okio.Buffer().writeLong(1).write(bytes),close={closed++})
            // Revision high byte 127 remains a valid positive Long; all other mutations are invalid.
            val result=runCatching { link.send(java.nio.ByteBuffer.allocate(1),0,1,0) }
            if (index==0) { assertTrue(result.isSuccess);continue }
            assertTrue(result.exceptionOrNull() is CameraTransportException);assertEquals(1,closed)
            assertThrows(IllegalStateException::class.java) { link.send(java.nio.ByteBuffer.allocate(1),0,1,0) }
        }
    }
    @Test fun t1468_liveBitrateConfigurationRespectsBoundsAndAdaptiveState() {
        val rate=CameraRate(3000,1000,true,150000)
        assertEquals(2250,rate.congested(1));assertNull(rate.configure(4000,1000,true,2) {})
        assertEquals(2000,rate.configure(2000,1000,true,3) {})
        assertEquals(5000,rate.configure(5000,1000,false,4) {});assertNull(rate.congested(5))
        for (value in listOf(255,20001)) {
            assertThrows(IllegalArgumentException::class.java) { rate.configure(value,1000,true,6) {} }
            assertEquals(5000,rate.current)
        }
        assertThrows(IllegalArgumentException::class.java) { rate.configure(3000,255,true,6) {} }
        assertThrows(IllegalArgumentException::class.java) { rate.configure(3000,1000,true,-1) {} }
        assertEquals(1000,rate.configure(1000,3000,true,7) {})
    }
    @Test fun t1476_partialNativeFailureCannotAcknowledgeARevertWithoutRestoringTheSensor() {
        var actual = CameraSensorTuning()
        var failEncoding = false
        val live = CameraLiveTuning(CameraTuningCapabilities(-2, 2, 1, 1, false, 1, 1),
            { actual = it }, { if (failEncoding) throw IllegalStateException("encoder rejected") })
        live.accept(command())
        failEncoding = true
        live.accept(command(2).copy(sensor = CameraSensorTuning(exposureHundredths = 100), bitrate = 2000))
        assertEquals(2, live.status)
        assertEquals(100, actual.exposureHundredths)
        failEncoding = false
        live.accept(command(3))
        assertEquals(0, live.status)
        assertEquals("T1476 success must describe the actual restored sensor", CameraSensorTuning(), actual)
    }

}
