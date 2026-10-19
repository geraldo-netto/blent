package com.blent.connection

/** Numeric settings only; device identity, trust, permission and native ownership
 * never enter this schema. Existing typed limits remain the validation owner. */
internal enum class ConnectionPreferenceField(
    val key: String,
    private val select: (ConnectionWorkerSpec) -> Long,
    private val replace: (ConnectionWorkerSpec, Long) -> ConnectionWorkerSpec,
) {
    CHANNEL_BYTES("channel_bytes", { it.session.capacity.toLong() }, { spec, value -> spec.copy(session = spec.session.copy(capacity = value.toInt())) }),
    QUEUED_MESSAGES("queued_messages", { it.session.messages.toLong() }, { spec, value -> spec.copy(session = spec.session.copy(messages = value.toInt())) }),
    QUANTUM("quantum", { it.session.quantum.toLong() }, { spec, value -> spec.copy(session = spec.session.copy(quantum = value.toInt())) }),
    RESOURCE_TIMEOUT("resource_timeout_ms", { it.session.resourceTimeoutMs }, { spec, value -> spec.copy(session = spec.session.copy(resourceTimeoutMs = value)) }),
    IO_TIMEOUT("io_timeout_ms", { it.io.ioTimeoutMs }, { spec, value -> spec.copy(io = spec.io.copy(ioTimeoutMs = value)) }),
    RETIREMENT_TIMEOUT("retirement_timeout_ms", { it.io.retirementTimeoutMs }, { spec, value -> spec.copy(io = spec.io.copy(retirementTimeoutMs = value)) }),
    APPROVAL_TIMEOUT("approval_timeout_ms", { it.approvalTimeoutMs }, { spec, value -> spec.copy(approvalTimeoutMs = value) }),
    NATIVE_TIMEOUT("native_timeout_ms", { it.nativeTimeoutMs }, { spec, value -> spec.copy(nativeTimeoutMs = value) }),
    TRUSTED_PEERS("trusted_peers", { it.trustedPeers.toLong() }, { spec, value -> spec.copy(trustedPeers = value.toInt()) }),
    ACCESSORY_CAPACITY("accessory_capacity", { it.accessoryCapacity.toLong() }, { spec, value -> spec.copy(accessoryCapacity = value.toInt()) }),
    FALLBACK("fallback_ms", { it.timerMs }, { spec, value -> spec.copy(timerMs = value) });

    fun value(spec: ConnectionWorkerSpec): String = select(spec).toString()

    /** At most seven decimal digits: conversion to Int cannot truncate. */
    fun update(spec: ConnectionWorkerSpec, text: String): ConnectionWorkerSpec {
        require(text.length in 1..7 && text.all { it in '0'..'9' }) { "Use 1–7 decimal digits for $key" }
        return replace(spec, text.toLong())
    }
}

/** One immutable next-connection snapshot. Parsing retains no input map/view and
 * never changes an active owner. Text persistence is bounded and deterministic. */
internal object ConnectionPreferences {
    const val MAX_ENCODING = 512

    fun fields(spec: ConnectionWorkerSpec): Map<String, String> =
        ConnectionPreferenceField.entries.associate { it.key to it.value(spec) }

    fun parse(fields: Map<String, String>, device: String = "usb"): ConnectionWorkerSpec {
        require(fields.size == ConnectionPreferenceField.entries.size) { "Incomplete USB settings" }
        var result = ConnectionWorkerSpec(device)
        for (field in ConnectionPreferenceField.entries) {
            result = field.update(result, requireNotNull(fields[field.key]) { "Unknown or missing USB setting" })
        }
        return result
    }

    fun encode(spec: ConnectionWorkerSpec): String = "1\n" + ConnectionPreferenceField.entries.joinToString("\n") {
        "${it.key}=${it.value(spec)}"
    }

    fun decode(encoded: String, device: String = "usb"): ConnectionWorkerSpec {
        require(encoded.length in 1..MAX_ENCODING) { "Invalid USB settings size" }
        val lines = encoded.split('\n', limit = ConnectionPreferenceField.entries.size + 2)
        require(lines.size == ConnectionPreferenceField.entries.size + 1 && lines.first() == "1") { "Invalid USB settings schema" }
        val fields = HashMap<String, String>(ConnectionPreferenceField.entries.size)
        for (line in lines.drop(1)) {
            val pair = line.split('=', limit = 2)
            require(pair.size == 2) { "Invalid USB setting" }
            require(fields.put(pair[0], pair[1]) == null) { "Duplicate USB setting" }
        }
        return parse(fields, device)
    }
}
