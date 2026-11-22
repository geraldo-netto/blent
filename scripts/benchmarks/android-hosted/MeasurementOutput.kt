package com.blent.connection

import java.io.File

/** One current hosted output-path contract, resolved before measured work starts. */
internal fun measurementOutput(path: String): File {
    val output = File(path).absoluteFile
    requireNotNull(output.parentFile) { "Output must name a file below a directory" }.mkdirs()
    return output
}
