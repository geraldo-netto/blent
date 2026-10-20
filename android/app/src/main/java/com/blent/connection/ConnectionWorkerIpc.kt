package com.blent.connection

import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import android.os.Process

/** Same-UID Android adapter for bounded private connection metadata only. */
internal object ConnectionWorkerIpc {
    const val REQUEST = "com.blent.connection.worker.1"
    const val EVENT = "com.blent.connection.worker.event.1"
    const val START = 1
    const val CONTROL = 2
    const val STOP = 3
    const val EXIT = 4
    const val START_TRUST = 5
    const val MAX_PARCEL = 8192
    fun validate(data: Parcel, descriptor: String, flags: Int) {
        check(Binder.getCallingUid() == Process.myUid()) { "Foreign connection worker caller" }
        // Native Binder adds transport metadata such as TF_ACCEPT_FDS (16).
        // Require asynchronous delivery; those extra bits grant no authority.
        require((flags and IBinder.FLAG_ONEWAY) != 0)
        require(data.dataSize() in 1..MAX_PARCEL)
        data.enforceInterface(descriptor)
    }
    fun generation(data: Parcel): Long = data.readLong().also { require(it > 0) }
    fun bytes(data: Parcel): ByteArray = requireNotNull(data.createByteArray()).also {
        require(it.size in 2..ConnectionWorkerCodec.LIMIT)
    }
    fun end(data: Parcel) { require(data.dataAvail() == 0) { "Trailing connection IPC data" } }
    fun command(remote: IBinder, code: Int, generation: Long, bytes: ByteArray? = null, callback: IBinder? = null) {
        require(code in START..START_TRUST && generation > 0)
        val data = Parcel.obtain()
        try {
            data.writeInterfaceToken(REQUEST); data.writeLong(generation)
            if (code == START || code == CONTROL || code == START_TRUST) {
                requireNotNull(bytes); require(bytes.size in 2..ConnectionWorkerCodec.LIMIT); data.writeByteArray(bytes)
            }
            if (code == START || code == START_TRUST) data.writeStrongBinder(requireNotNull(callback))
            send(remote, code, data)
        } finally { data.recycle() }
    }
    fun event(remote: IBinder, generation: Long, event: ConnectionWorkerEvent) {
        require(generation > 0)
        val data = Parcel.obtain()
        try {
            data.writeInterfaceToken(EVENT); data.writeLong(generation)
            data.writeByteArray(ConnectionWorkerCodec.event(event)); send(remote, 1, data)
        } finally { data.recycle() }
    }
    private fun send(remote: IBinder, code: Int, data: Parcel) {
        require(data.dataSize() <= MAX_PARCEL)
        check(remote.transact(code, data, null, IBinder.FLAG_ONEWAY)) { "Connection IPC rejected" }
    }
}
