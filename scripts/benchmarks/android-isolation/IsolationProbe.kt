package com.blent.benchmark

import android.content.*
import android.graphics.SurfaceTexture
import android.os.*
import android.view.Surface
import org.json.*
import java.io.File
import java.util.concurrent.*
import java.util.concurrent.atomic.*

/** Shell-only acceptance runner. It can terminate only its own disposable workers. */
class IsolationProbe : BroadcastReceiver() {
    companion object { private val running = AtomicBoolean() }
    override fun onReceive(context: Context, intent: Intent) {
        if (!running.compareAndSet(false, true)) return
        val completion = goAsync()
        Thread {
            try {
                val result = try { run(context.applicationContext, intent.getBooleanExtra("grouped", false)) }
                    catch (error: Throwable) { JSONObject().put("error", error.stackTraceToString()) }
                result.put("run_id", intent.getStringExtra("run_id"))
                File(context.filesDir, "isolation.json").writeText(result.toString(2))
            } finally { running.set(false); completion.finish() }
        }.start()
    }
    private fun run(context: Context, grouped: Boolean): JSONObject {
        val types = if (grouped) listOf(Grouped0::class.java, Grouped1::class.java)
            else listOf(Dedicated0::class.java, Dedicated1::class.java, Dedicated2::class.java, Dedicated3::class.java)
        val peers = ArrayList<Peer>()
        val rows = JSONArray()
        try {
            for ((index, type) in types.withIndex()) {
                val started = System.nanoTime()
                val peer = Peer(context, type, index + 1L).also { peers.add(it) }
                val reply = peer.call(1)
                check(reply.pid != Process.myPid())
                rows.put(JSONObject().put("pid", reply.pid).put("pss_kb", reply.pss).put("rss_kb", reply.rss).put("cpu_ms", reply.cpu)
                    .put("startup_ns", System.nanoTime() - started))
            }
            val unique = peers.map { it.pid }.toSet()
            check(unique.size == if (grouped) 1 else 4)
            // Bounded one-operation admission: held work rejects a second request.
            check(peers[0].call(2).result == 2)
            check(peers[0].call(1).result == -1)
            check(peers[1].call(1).result == 1)
            check(peers[0].call(3).result == 3)
            // Surface handle ownership transfer, not a pixel parcel.
            val texture = SurfaceTexture(false)
            val surface = Surface(texture)
            try {
                check(peers[1].call(4, surface).result == 4)
                check(surface.isValid)
                check(peers[1].call(5).result == 5)
                check(surface.isValid)
            } finally { surface.release(); texture.release() }
            // Native-owner simulation stays held during death; its sibling remains live.
            var admitted = false
            repeat(10) {
                if (!admitted) { admitted = peers[0].call(2).result == 2; if (!admitted) Thread.sleep(10) }
            }
            check(admitted)
            peers[0].send(9)
            check(peers[0].dead.await(3, TimeUnit.SECONDS))
            if (grouped) check(peers[1].dead.await(3, TimeUnit.SECONDS))
            else check(peers[1].call(1).result == 1)
            val oldPid = peers[0].pid
            // Detach every dead generation before explicit reconnect; no replay.
            if (grouped) peers.forEach { it.close() } else peers[0].close()
            val restarted = Peer(context, types[0], 100).also { peers.add(it) }
            val reply = restarted.call(1)
            check(reply.pid != oldPid)
            // Reject a stale reply locally without publishing it to current work.
            check(restarted.rejectStale())
            return JSONObject().put("fingerprint", Build.FINGERPRINT).put("grouped", grouped)
                .put("workers", rows).put("restarted_pid", reply.pid).put("death_contained", !grouped)
                .put("hang_admission_bounded", true).put("surface_handle_transfer", true).put("stale_reply_rejected", true)
        } finally { peers.forEach { it.close() } }
    }
}

private data class Reply(val result: Int, val pid: Int, val pss: Int, val rss: Int, val cpu: Long)
private class Peer(private val context: Context, type: Class<*>, private val generation: Long) : AutoCloseable {
    val dead = CountDownLatch(1)
    private val connected = CountDownLatch(1)
    private val replies = ArrayBlockingQueue<Reply>(1)
    private val pending = AtomicBoolean()
    private var bound = false
    private lateinit var remote: IBinder
    var pid = -1
        private set
    private val callback = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            check(getCallingUid() == Process.myUid())
            data.enforceInterface("blent.probe.reply.1")
            val revision = data.readLong()
            val value = Reply(data.readInt(), data.readInt(), data.readInt(), data.readInt(), data.readLong())
            if (revision == generation && pending.compareAndSet(true, false)) check(replies.offer(value))
            return true
        }
    }
    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, binder: IBinder) {
            remote = binder
            binder.linkToDeath({ dead.countDown() }, 0)
            connected.countDown()
        }
        override fun onServiceDisconnected(name: ComponentName) { dead.countDown() }
    }
    init {
        bound = context.bindService(Intent(context, type), connection, Context.BIND_AUTO_CREATE)
        check(bound)
        try { check(connected.await(5, TimeUnit.SECONDS)) } catch (error: Exception) { close(); throw error }
    }
    fun call(code: Int, surface: Surface? = null): Reply {
        check(pending.compareAndSet(false, true))
        send(code, surface)
        val result = checkNotNull(replies.poll(5, TimeUnit.SECONDS)) { "Worker callback timeout" }
        pid = result.pid
        return result
    }
    fun send(code: Int, surface: Surface? = null) {
        val data = Parcel.obtain()
        try {
            data.writeInterfaceToken("blent.probe.1"); data.writeLong(generation); data.writeStrongBinder(callback)
            surface?.writeToParcel(data, 0)
            check(remote.transact(code, data, null, IBinder.FLAG_ONEWAY))
        } finally { data.recycle() }
    }
    fun rejectStale(): Boolean {
        val data = Parcel.obtain()
        try {
            data.writeInterfaceToken("blent.probe.reply.1"); data.writeLong(generation - 1)
            data.writeInt(1); data.writeInt(-1); data.writeInt(0); data.writeInt(0); data.writeLong(0)
            check(pending.compareAndSet(false, true))
            data.setDataPosition(0)
            callback.transact(1, data, null, 0)
            return replies.isEmpty() && pending.get()
        } finally { pending.set(false); data.recycle() }
    }
    override fun close() {
        if (bound) { bound = false; context.unbindService(connection) }
    }
}
