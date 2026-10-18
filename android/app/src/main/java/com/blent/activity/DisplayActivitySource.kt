package com.blent.activity

import org.json.JSONObject

/** One control-lifetime owner of the original inbox and its replacement history.
 * The control actor serializes writes; readers retain only this pure view owner,
 * without sockets, callbacks, UI objects or native resources. */
internal class DisplayActivitySource : DisplayActivityView {
    @Volatile private var inbox = DisplayActivityInbox()
    override val update get() = inbox.update
    override fun current(snapshot: DisplayActivityUpdate): Boolean = update === snapshot
    fun receive(message: JSONObject): Boolean = inbox.receive(message)
    fun close() = inbox.close()
    fun replace() {
        inbox.close()
        inbox = DisplayActivityInbox(inbox.update.supportKnown)
    }
}
