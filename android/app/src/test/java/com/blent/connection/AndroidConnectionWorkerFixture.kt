package com.blent.connection

import android.content.*
import android.os.*
import org.robolectric.RuntimeEnvironment
import java.util.concurrent.CopyOnWriteArrayList

internal class ConnectionWorkerEndpoint : Binder() {
    val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
    val codes = CopyOnWriteArrayList<Int>()
    val controls = CopyOnWriteArrayList<ConnectionWorkerCommand>()
    @Volatile var generation = 0L
    @Volatile var callback: IBinder? = null
    @Volatile var alive = true
    var failLink = false
    var links = 0
    var killOnExit = true
    var start: (ConnectionWorkerEndpoint) -> Unit = { it.emit(ConnectionWorkerEvent.Ended(null)) }
    var control: (ConnectionWorkerEndpoint) -> Unit = {}
    var stop: () -> Unit = {}
    override fun isBinderAlive(): Boolean = alive
    override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) {
        links++
        if (failLink) throw RemoteException("link rejected")
        deaths.add(recipient)
    }
    override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int): Boolean = deaths.remove(recipient)
    override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
        ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, flags)
        generation = ConnectionWorkerIpc.generation(data); codes.add(code)
        when (code) {
            ConnectionWorkerIpc.START -> {
                ConnectionWorkerCodec.spec(ConnectionWorkerIpc.bytes(data))
                callback = data.readStrongBinder(); ConnectionWorkerIpc.end(data); start(this)
            }
            ConnectionWorkerIpc.CONTROL -> {
                controls.add(ConnectionWorkerCodec.command(ConnectionWorkerIpc.bytes(data)))
                ConnectionWorkerIpc.end(data); control(this)
            }
            ConnectionWorkerIpc.STOP -> { ConnectionWorkerIpc.end(data); stop() }
            ConnectionWorkerIpc.EXIT -> { ConnectionWorkerIpc.end(data); if (killOnExit) die() }
        }
        return true
    }
    fun die() { alive = false; deaths.toList().forEach { it.binderDied() } }
    fun emit(event: ConnectionWorkerEvent) = ConnectionWorkerIpc.event(checkNotNull(callback), generation, event)
    fun raw(bytes: ByteArray, revision: Long = generation, trailing: Boolean = false): Boolean {
        val data = Parcel.obtain()
        try {
            data.writeInterfaceToken(ConnectionWorkerIpc.EVENT); data.writeLong(revision); data.writeByteArray(bytes)
            if (trailing) data.writeInt(9)
            return checkNotNull(callback).transact(1, data, null, IBinder.FLAG_ONEWAY)
        } finally { data.recycle() }
    }
}

internal class ConnectionWorkerContext(val endpoint: ConnectionWorkerEndpoint) : ContextWrapper(RuntimeEnvironment.getApplication()) {
    @Volatile var binds = 0
    @Volatile var unbinds = 0
    @Volatile var connection: ServiceConnection? = null
    lateinit var component: ComponentName
    var accepts = true
    var bindFailure = false
    var connect = true
    var nullBinding = false
    var unbindFailure = false
    var checkIntent: (Intent) -> Unit = {}
    var duringUnbind: () -> Unit = {}
    override fun getApplicationContext(): Context = this
    override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
        this.connection = connection; component = checkNotNull(intent.component); binds++
        checkIntent(intent)
        if (bindFailure) throw SecurityException("bind denied")
        if (!accepts) return false
        if (nullBinding) {
            // API 27 has no onNullBinding; its nullable connected callback exercises rejection.
            if (Build.VERSION.SDK_INT >= 28) connection.onNullBinding(component)
            else connection.onServiceConnected(component, null)
        } else if (connect) connection.onServiceConnected(component, endpoint)
        return true
    }
    override fun unbindService(connection: ServiceConnection) {
        unbinds++
        duringUnbind()
        if (unbindFailure) throw IllegalArgumentException("unbind rejected")
    }
}
