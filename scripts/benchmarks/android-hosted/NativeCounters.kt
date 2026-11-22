package com.blent.connection

internal data class Counters(val cpu: Long, val allocated: Long?)
/** Hosted JDK API adapter; Android compilation deliberately keeps its normal boot classpath. */
internal class NativeCounters {
    private val factory = Class.forName("java.lang.management.ManagementFactory")
    private val threadContract = Class.forName("com.sun.management.ThreadMXBean")
    private val processContract = Class.forName("com.sun.management.OperatingSystemMXBean")
    private val bean = factory.getMethod("getThreadMXBean").invoke(null)
    private val process = factory.getMethod("getOperatingSystemMXBean").invoke(null)
    private val cpu = threadContract.getMethod("getThreadCpuTime", java.lang.Long.TYPE)
    private val allocated = threadContract.getMethod("getThreadAllocatedBytes", java.lang.Long.TYPE)
    private val processCpu = processContract.getMethod("getProcessCpuTime")
    private val allocationSupported = flag("isThreadAllocatedMemorySupported")
    init {
        check(flag("isThreadCpuTimeSupported")) { "T918 thread CPU counters unavailable" }
        enable("setThreadCpuTimeEnabled")
        if (allocationSupported) enable("setThreadAllocatedMemoryEnabled")
    }
    private fun flag(name: String): Boolean = threadContract.getMethod(name).invoke(bean) as Boolean
    private fun enable(name: String) {
        threadContract.getMethod(name, java.lang.Boolean.TYPE).invoke(bean, true)
    }
    fun read(thread: Thread): Counters {
        val time = cpu.invoke(bean, thread.id) as Long
        check(time >= 0) { "T918 original thread CPU counter unavailable" }
        val bytes = if (allocationSupported) allocated.invoke(bean, thread.id) as Long else null
        check(bytes == null || bytes >= 0)
        return Counters(time, bytes)
    }
    fun processCpu(): Long = (processCpu.invoke(process) as Long).also {
        check(it >= 0) { "T918 JVM process CPU counter unavailable" }
    }
}

