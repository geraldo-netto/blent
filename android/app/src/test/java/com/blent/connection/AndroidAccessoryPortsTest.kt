package com.blent.connection

import android.os.ParcelFileDescriptor
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class AndroidAccessoryPortsTest {
    private fun descriptor(action: (File, ParcelFileDescriptor) -> Unit) {
        val file = File.createTempFile("blent-accessory", ".test")
        file.writeBytes(byteArrayOf(1, 2, 3))
        val descriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_WRITE)
        try { action(file, descriptor) } finally { descriptor.close(); file.delete() }
    }
    @Test fun t846NativeDescriptorStreamsOwnIndependentReferences() = descriptor { _, descriptor ->
        val ports = AndroidAccessoryPorts.open(descriptor)
        val bytes = ByteArray(65536)
        assertEquals(3, ports.input.read(bytes))
        assertArrayEquals(byteArrayOf(1, 2, 3), bytes.copyOf(3))
        ports.output.write(byteArrayOf(4, 5))
        ports.close(); ports.close()
        assertThrows(IllegalStateException::class.java) { descriptor.fd }
    }
    @Test fun t846DuplicateAndInputCreationFailureCloseOriginal() {
        descriptor { _, descriptor ->
            assertThrows(IOException::class.java) {
                AndroidAccessoryPorts.open(descriptor, duplicate = { throw IOException("Dup failed") })
            }
            assertThrows(IllegalStateException::class.java) { descriptor.fd }
        }
        descriptor { _, descriptor ->
            var copied: ParcelFileDescriptor? = null
            assertThrows(IOException::class.java) {
                AndroidAccessoryPorts.open(descriptor, duplicate = {
                    ParcelFileDescriptor.dup(it).also { owned -> copied = owned }
                }, input = { throw IOException("Input wrapper failed") })
            }
            assertThrows(IllegalStateException::class.java) { descriptor.fd }
            assertThrows(IllegalStateException::class.java) { copied!!.fd }
        }
    }
    @Test fun t846OutputCreationFailureClosesCreatedInputAndOriginal() = descriptor { _, descriptor ->
        var closed = false
        val reader = object : ByteArrayInputStream(byteArrayOf()) {
            override fun close() { closed = true; throw IOException("Close failure") }
        }
        var copied: ParcelFileDescriptor? = null
        try {
            val failure = assertThrows(IOException::class.java) {
                AndroidAccessoryPorts.open(descriptor, duplicate = {
                    ParcelFileDescriptor.dup(it).also { owned -> copied = owned }
                }, input = { reader }, output = { throw IOException("Output wrapper failed") })
            }
            assertEquals("Output wrapper failed", failure.message)
            assertEquals(1, failure.suppressed.size); assertTrue(closed)
            assertThrows(IllegalStateException::class.java) { descriptor.fd }
            assertThrows("T846: failed stream close must not leak its duplicated descriptor", IllegalStateException::class.java) { copied!!.fd }
        } finally { copied?.close() }
    }
    @Test fun t846CloseIsSingleOwnerAndAttemptsBothStreams() = descriptor { _, descriptor ->
        var inputCloses = 0
        var outputCloses = 0
        val ports = AndroidAccessoryPorts.open(descriptor,
            input = { owned -> object : ByteArrayInputStream(byteArrayOf()) {
                override fun close() { inputCloses++; owned.close(); throw IOException("Close failed") }
            } },
            output = { owned -> object : ByteArrayOutputStream() {
                override fun close() { outputCloses++; owned.close() }
            } },
        )
        assertThrows(IOException::class.java) { ports.close() }
        ports.close()
        assertEquals(1, inputCloses); assertEquals(1, outputCloses)
    }
}
