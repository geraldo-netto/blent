package com.blent

/** Opaque original foreground identity; retained references cannot compare equal
 * to a later allocation, independently of wire/process counters. */
internal class DisplayPowerOwner internal constructor()
internal class DisplayPowerRequest internal constructor(
    val owner: DisplayPowerOwner,
    val power: StreamingPower?,
    val permit: DisplayPowerPermit = FixedDisplayPowerPermit,
)
internal class DisplayPowerNotice internal constructor(val original: DisplayPowerRequest)
internal class DisplayPowerAdmission internal constructor(val owner: DisplayPowerOwner, val notice: DisplayPowerNotice?)
internal class DisplayPowerDelivery internal constructor(val accepted: Boolean, val request: DisplayPowerRequest?, val next: DisplayPowerNotice?)

/** Foreground producer owns desired work; null request power withdraws it. */
internal interface DisplayPowerView {
    fun owns(owner: DisplayPowerOwner): Boolean
    fun current(request: DisplayPowerRequest): Boolean
}

internal interface DisplayPowerPublisher : DisplayPowerView {
    fun open(power: StreamingPower, permit: DisplayPowerPermit = FixedDisplayPowerPermit): DisplayPowerAdmission
    fun publish(owner: DisplayPowerOwner, power: StreamingPower, permit: DisplayPowerPermit = FixedDisplayPowerPermit): DisplayPowerNotice?
    fun close(owner: DisplayPowerOwner): DisplayPowerNotice?
}

/** Original notice receipt and exact latest-view relevance. Neither operation
 * is native lock/decoder permission or proof of native release. */
internal interface DisplayPowerReceiver : DisplayPowerView {
    fun pending(notice: DisplayPowerNotice): Boolean
    fun receive(notice: DisplayPowerNotice): DisplayPowerDelivery
}

internal interface DisplayPowerChannel : DisplayPowerPublisher, DisplayPowerReceiver
