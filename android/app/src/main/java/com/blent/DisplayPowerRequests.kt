package com.blent

/** One latest desired policy and one original queued notice. Native transport
 * receipt only settles notice admission, never lock/decoder ownership. */
internal class DisplayPowerRequests : DisplayPowerChannel {
    private var latest: DisplayPowerRequest? = null
    private var queued: DisplayPowerNotice? = null
    private var dirty = false

    @Synchronized override fun open(power: StreamingPower, permit: DisplayPowerPermit): DisplayPowerAdmission {
        val owner = DisplayPowerOwner()
        latest = DisplayPowerRequest(owner, power, permit)
        dirty = true
        return DisplayPowerAdmission(owner, admit())
    }

    @Synchronized override fun publish(owner: DisplayPowerOwner, power: StreamingPower, permit: DisplayPowerPermit): DisplayPowerNotice? {
        if (!owns(owner)) return null
        latest = DisplayPowerRequest(owner, power, permit)
        dirty = true
        return admit()
    }

    @Synchronized override fun close(owner: DisplayPowerOwner): DisplayPowerNotice? {
        if (!owns(owner)) return null
        latest = DisplayPowerRequest(owner, null)
        dirty = true
        return admit()
    }

    @Synchronized override fun owns(owner: DisplayPowerOwner): Boolean = latest?.let { it.owner === owner && it.power != null } == true
    @Synchronized override fun current(request: DisplayPowerRequest): Boolean = latest === request
    @Synchronized override fun pending(notice: DisplayPowerNotice): Boolean = queued === notice

    @Synchronized override fun receive(notice: DisplayPowerNotice): DisplayPowerDelivery {
        if (queued !== notice) return DisplayPowerDelivery(false, null, null)
        queued = null
        val request = take(notice.original.owner)
        return DisplayPowerDelivery(true, request, admit())
    }

    private fun take(owner: DisplayPowerOwner): DisplayPowerRequest? {
        val request = latest?.takeIf { it.owner === owner } ?: return null
        dirty = false
        return request
    }

    private fun admit(): DisplayPowerNotice? {
        if (queued != null || !dirty) return null
        val request = latest ?: return null
        return DisplayPowerNotice(request).also { queued = it }
    }
}
