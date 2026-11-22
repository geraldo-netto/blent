package com.blent.connection

import java.io.File
import java.lang.reflect.InvocationTargetException
import java.nio.file.Files
import java.util.UUID
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class IdleOutputTest {
    private fun invoke(entry: String, output: String) {
        try {
            Class.forName("com.blent.connection.$entry").getMethod("main", Array<String>::class.java)
                .invoke(null, arrayOf(output) as Any)
        } catch (failure: InvocationTargetException) { throw failure.targetException }
    }
    private fun bare(entry: String, trials: Int) {
        val output = File("t1769-${UUID.randomUUID()}.json")
        assertFalse(output.exists())
        try {
            invoke(entry, output.path)
            val report = Json.parseToJsonElement(output.readText()).jsonObject
            assertEquals(trials, report.getValue("trials").jsonArray.size)
        } finally { Files.deleteIfExists(output.toPath()) }
    }
    @Test fun t1769AllocationBenchmarkAcceptsBareFilename() {
        bare("DeadlineAllocationBenchmarkKt", 4)
    }
    @Test fun t1769ActorBenchmarkAcceptsBareFilename() {
        bare("ConnectionIdleBenchmarkKt", 7)
    }
    @Test fun t1769RootOutputIsRejectedByBothEntrypoints() {
        for (entry in listOf("ConnectionIdleBenchmarkKt", "DeadlineAllocationBenchmarkKt")) {
            assertThrows(IllegalArgumentException::class.java) { invoke(entry, File.listRoots().first().path) }
        }
    }
}
