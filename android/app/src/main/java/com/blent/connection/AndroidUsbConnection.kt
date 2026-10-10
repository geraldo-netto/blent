package com.blent.connection

import android.content.Context
import android.os.SystemClock

/** Main-process entry point. Template device is replaced only after fresh native
 * permission; no caller-supplied device string can bypass admission. UI owns one
 * run and must await actual retirement before constructing its replacement. */
internal fun androidUsbConnection(
    context: () -> Context,
    spec: ConnectionWorkerSpec,
    report: (ConnectionWorkerEvent) -> Unit,
    clock: () -> Long = SystemClock::elapsedRealtime,
): ConnectionWorker = ConnectionAdmissionRun(spec, { wake ->
    AccessoryAdmissionOwner(
        AndroidAccessoryAdmissionBackend(context, AndroidConnectionFactory.profile, spec.accessoryCapacity),
        AndroidConnectionFactory.profile, spec.accessoryCapacity, clock,
        spec.nativeTimeoutMs, spec.approvalTimeoutMs, spec.io.retirementTimeoutMs, wake,
    )
}, { selected, events -> AndroidConnectionWorkerRun(context(), selected, events) }, report)
