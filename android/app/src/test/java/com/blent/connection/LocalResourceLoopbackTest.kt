package com.blent.connection

import java.io.DataInputStream
import java.net.Socket
import org.junit.Assert.*
import org.junit.Test

/** Real JVM loopback listeners/I/O plus production management/mux/pumps. This
 * cannot establish Android Binder/Activity or physical USB acceptance (T857). */
class LocalResourceLoopbackTest {
    @Test fun t906ProductionProtocolBridgesBothDisplayChannelsAndRetiresNativeWorkers() {
        val rig = TabletSessionRig(capabilities = 1, timeout = 60000)
        val local = ConnectionLocalResources(rig.tablet, 4096)
        val sockets = ArrayList<Socket>()
        val origin = System.nanoTime()
        fun step() {
            rig.now = (System.nanoTime() - origin) / 1_000_000
            check(rig.now < 5000) { "T906 real local bridge stalled" }
            local.advance(rig.now); rig.pump(); Thread.sleep(1)
        }
        try {
            val offer = rig.invitation(); rig.send(offer)
            while (rig.messages.none { it == ConnectionManagement.Prepared(offer.key) }) step()
            assertNull(local.display()); rig.send(ConnectionManagement.Commit(offer.key))
            rig.host.activate(1, 1); rig.host.activate(2, 1); step()
            val route = checkNotNull(local.display())
            assertEquals(offer.key, route.lease.key)
            for (port in listOf(route.controlPort, route.displayPort)) {
                sockets.add(Socket("127.0.0.1", port).apply { soTimeout = 1000 })
            }
            for ((index, socket) in sockets.withIndex()) {
                socket.getOutputStream().write(byteArrayOf(index.toByte(), 5, 6))
                assertTrue(rig.host.offer(index + 1, byteArrayOf(7, 8, index.toByte())))
            }
            while (rig.payloads.sumOf { it.second.size } < 6) step()
            for ((index, socket) in sockets.withIndex()) {
                val bytes = ByteArray(3); DataInputStream(socket.getInputStream()).readFully(bytes)
                assertArrayEquals(byteArrayOf(7, 8, index.toByte()), bytes)
                assertArrayEquals(byteArrayOf(index.toByte(), 5, 6),
                    rig.payloads.filter { it.first == index + 1 }.flatMap { it.second.toList() }.toByteArray())
            }
            rig.tablet.cancel(route.lease, ConnectionManagement.Reason.REQUESTED)
            assertNull(local.display())
        } finally {
            local.stop(rig.now); rig.tablet.disconnect(); rig.tablet.transportRetired()
            sockets.forEach { it.close() }
            while (!local.retired()) {
                rig.now = (System.nanoTime() - origin) / 1_000_000
                check(rig.now < 7000) { "T906 real local ownership did not retire" }
                local.advance(rig.now); Thread.sleep(1)
            }
        }
        assertFalse(local.quarantined())
    }
}
