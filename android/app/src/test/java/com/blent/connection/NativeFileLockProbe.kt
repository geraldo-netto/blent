package com.blent.connection

import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.TimeUnit

/** Exact original test inode; independent child, actual exit/reap, all pipes closed. */
internal object NativeFileLockProbe {
    data class Identity(val device: String, val inode: String)
    fun identity(file: Path): Identity {
        val values = Files.readAttributes(file, "unix:dev,ino")
        return Identity(values["dev"].toString(), values["ino"].toString())
    }
    fun held(file: Path, original: Identity): Int {
        val program = file.parent.resolve("probe.py")
        javaClass.classLoader!!.getResourceAsStream("trust-lock-probe.py")!!.use { source ->
            Files.newOutputStream(program).use { source.copyTo(it) }
        }
        val process = ProcessBuilder("/usr/bin/python3", program.toString(), file.toString(), original.device, original.inode).start()
        return try {
            check(process.waitFor(5, TimeUnit.SECONDS)) { "T1157 native probe protocol failed" }
            process.exitValue()
        } finally { settle(process) }
    }
    private fun settle(process: Process) {
        try {
            if (process.isAlive) process.destroyForcibly()
            process.waitFor()
        } finally { closeConnectionResources(listOf(process.inputStream, process.errorStream, process.outputStream)) }
    }
}
