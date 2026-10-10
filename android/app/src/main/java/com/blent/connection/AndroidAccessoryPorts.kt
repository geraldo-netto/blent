package com.blent.connection

import android.os.ParcelFileDescriptor
import java.io.Closeable
import java.io.FileDescriptor
import java.io.InputStream
import java.io.OutputStream
import java.util.concurrent.atomic.AtomicBoolean

/** Owns both descriptor references. Opening transfers the caller's descriptor
 * even on failure. Passing both streams to an admitted SocketBridge transfers
 * ownership again; do not concurrently close this wrapper afterward. All native
 * operations, including close, require the supervised connection worker. */
internal class AndroidAccessoryPorts private constructor(
    val input: InputStream,
    val output: OutputStream,
) : Closeable {
    private val closing = AtomicBoolean()
    companion object {
        fun open(
            descriptor: ParcelFileDescriptor,
            duplicate: (FileDescriptor) -> ParcelFileDescriptor = ParcelFileDescriptor::dup,
            input: (ParcelFileDescriptor) -> InputStream = { ParcelFileDescriptor.AutoCloseInputStream(it) },
            output: (ParcelFileDescriptor) -> OutputStream = { ParcelFileDescriptor.AutoCloseOutputStream(it) },
        ): AndroidAccessoryPorts {
            var copy: ParcelFileDescriptor? = null
            var reader: InputStream? = null
            try {
                copy = duplicate(descriptor.fileDescriptor)
                reader = input(copy)
                return AndroidAccessoryPorts(reader, output(descriptor))
            } catch (failure: Exception) {
                closeConnectionFailure(failure, listOfNotNull(reader, copy, descriptor))
                throw failure
            }
        }
    }
    override fun close() {
        if (closing.compareAndSet(false, true)) closeConnectionResources(listOf(input, output))
    }
}
