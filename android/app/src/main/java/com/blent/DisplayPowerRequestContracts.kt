package com.blent

/** Opaque original foreground identity; retained references cannot compare equal
 * to a later allocation, independently of wire/process counters. */
internal class DisplayPowerOwner internal constructor()
internal class DisplayPowerRequest internal constructor(val owner: DisplayPowerOwner, val power: StreamingPower?)
internal class DisplayPowerNotice internal constructor(val original: DisplayPowerRequest)
internal class DisplayPowerAdmission internal constructor(val owner: DisplayPowerOwner, val notice: DisplayPowerNotice?)
internal class DisplayPowerDelivery internal constructor(val accepted: Boolean, val request: DisplayPowerRequest?, val next: DisplayPowerNotice?)

/** Foreground producer owns desired work; null request power withdraws it. */
internal interface DisplayPowerPublisher {
    fun open(power: StreamingPower): DisplayPowerAdmission
    fun publish(owner: DisplayPowerOwner, power: StreamingPower): DisplayPowerNotice?
    fun close(owner: DisplayPowerOwner): DisplayPowerNotice?
    fun owns(owner: DisplayPowerOwner): Boolean
}

/** Original notice receipt and exact latest-view relevance. Neither operation
 * is native lock/decoder permission or proof of native release. */
internal interface DisplayPowerReceiver {
    fun receive(notice: DisplayPowerNotice): DisplayPowerDelivery
    fun current(request: DisplayPowerRequest): Boolean
}

internal interface DisplayPowerChannel : DisplayPowerPublisher, DisplayPowerReceiver
