package com.blent.connection

import java.nio.file.Files
import org.junit.Assert.*
import org.junit.Test

class TunnelOutputTest {
    @Test fun t1761FilesystemRootIsNotAnOutputFilename() {
        assertThrows(IllegalArgumentException::class.java) { measurementOutput(java.io.File.listRoots().first().path) }
    }

    @Test fun t1761BareOutputFilenameResolvesWithoutNullParent() {
        val name = "t1761-${java.util.UUID.randomUUID()}.json"
        val output = measurementOutput(name)
        assertTrue(output.isAbsolute)
        assertEquals(name, output.name)
        assertTrue(output.parentFile.isDirectory)
        assertFalse(output.exists())
    }
    @Test fun t1761AbsoluteOutputCreatesOnlyItsOwnedParentDirectory() {
        val directory = Files.createTempDirectory("blent-t1761-").toFile()
        try {
            val expected = directory.resolve("nested/result.json")
            val output = measurementOutput(expected.path)
            assertEquals(expected, output)
            assertTrue(output.parentFile.isDirectory)
            assertFalse(output.exists())
        } finally { check(directory.deleteRecursively()) }
    }
}
