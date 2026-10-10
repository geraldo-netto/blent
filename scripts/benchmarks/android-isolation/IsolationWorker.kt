package com.blent.benchmark

import android.app.Service
import android.content.Intent
import android.os.*
import android.view.Surface
import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/** Disposable prototype only. Commands carry bounded metadata, never raw frames.
 * One admitted native operation per owner; control remains outside that worker. */
open class IsolationWorker : Service() {
    private val busy = AtomicBoolean()
    private val held = AtomicReference<CountDownLatch?>(null)
    private var surface: Surface? = null
    private val binder = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            check(getCallingUid() == Process.myUid())
            data.enforceInterface("blent.probe.1")
            require(data.dataSize() <= 1024)
            val generation = data.readLong()
            val callback = requireNotNull(data.readStrongBinder())
            if (code == 9) { Process.killProcess(Process.myPid()); return true }
            if (code == 3) { held.get()?.countDown(); respond(callback, generation, 3); return true }
            if (!busy.compareAndSet(false, true)) { respond(callback, generation, -1); return true }
            val incoming = if (code == 4) Surface.CREATOR.createFromParcel(data) else null
            val stall = if (code == 2) CountDownLatch(1).also { held.set(it) } else null
            Thread {
                try {
                    if (code == 2) { respond(callback, generation, 2); stall!!.await() }
                    else {
                        val result = when (code) {
                            1 -> 1
                            4 -> { check(incoming!!.isValid); surface?.release(); surface = incoming; 4 }
                            5 -> { surface?.release(); surface = null; 5 }
                            else -> -2
                        }
                        busy.set(false)
                        respond(callback, generation, result)
                    }
                } finally { if (code == 2) busy.set(false) }
            }.start()
            return true
        }
    }
    private fun respond(callback: IBinder, generation: Long, result: Int) {
        val data = Parcel.obtain()
        try {
            data.writeInterfaceToken("blent.probe.reply.1"); data.writeLong(generation)
            data.writeInt(result); data.writeInt(Process.myPid())
            val memory = Debug.MemoryInfo().also { Debug.getMemoryInfo(it) }
            data.writeInt(memory.totalPss)
            val rss = java.io.File("/proc/self/status").readLines().first { it.startsWith("VmRSS:") }.split(Regex("\\s+"))[1].toInt()
            data.writeInt(rss); data.writeLong(Process.getElapsedCpuTime())
            callback.transact(1, data, null, IBinder.FLAG_ONEWAY)
        } finally { data.recycle() }
    }
    override fun onBind(intent: Intent): IBinder = binder
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int) = START_NOT_STICKY
    override fun onDestroy() { held.get()?.countDown(); surface?.release(); super.onDestroy() }
}
class Dedicated0 : IsolationWorker()
class Dedicated1 : IsolationWorker()
class Dedicated2 : IsolationWorker()
class Dedicated3 : IsolationWorker()
class Grouped0 : IsolationWorker()
class Grouped1 : IsolationWorker()
