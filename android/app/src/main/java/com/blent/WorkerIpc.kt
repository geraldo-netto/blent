package com.blent

import android.os.*
import android.view.Surface

internal object WorkerIpc {
    const val REQUEST = "com.blent.worker.1"
    const val EVENT = "com.blent.worker.event.1"
    const val START = 1
    const val STOP = 2
    const val EXIT = 3
    const val MAX_PARCEL = 600 * 1024

    fun validate(data: Parcel, descriptor: String) {
        check(Binder.getCallingUid() == Process.myUid()) { "Foreign worker caller" }
        require(data.dataSize() in 1..MAX_PARCEL) { "Worker parcel capacity exceeded" }
        data.enforceInterface(descriptor)
    }
    fun command(remote: IBinder, code: Int, generation: Long, task: String? = null, callback: IBinder? = null, surface: Surface? = null) {
        val data = Parcel.obtain()
        try {
            data.writeInterfaceToken(REQUEST); data.writeLong(generation)
            if (code == START) {
                data.writeString(task); data.writeStrongBinder(callback)
                data.writeInt(if (surface == null) 0 else 1)
                surface?.writeToParcel(data, 0)
            }
            require(data.dataSize() <= MAX_PARCEL)
            check(remote.transact(code, data, null, IBinder.FLAG_ONEWAY)) { "Worker command rejected" }
        } finally { data.recycle() }
    }
    fun event(remote: IBinder, generation: Long, text: String) {
        val data = Parcel.obtain()
        try {
            data.writeInterfaceToken(EVENT); data.writeLong(generation); data.writeString(text)
            require(data.dataSize() <= MAX_PARCEL)
            check(remote.transact(1, data, null, IBinder.FLAG_ONEWAY)) { "Worker event rejected" }
        } finally { data.recycle() }
    }
}
