package com.blent

import java.io.IOException
import org.junit.Assert.*
import org.junit.Test

class AoaSecureDiagnosticsTest {
    @Test fun t846CleanupFailureCannotReplacePrimaryTlsFailure() {
        val tls = IOException("TLS failed", IllegalStateException("Native provider failed"))
        val cleanup = IOException("Native read did not retire")
        val observed = AoaSecureDiagnostics.result(Result.failure(tls), Result.failure(cleanup))
        assertSame("T846: preserve handshake failure when native retirement also fails", tls, observed.exceptionOrNull())
    }
    @Test fun t846CleanupFailureStillPreventsSuccessfulQualification() {
        val cleanup = IOException("Native read did not retire")
        assertSame(cleanup, AoaSecureDiagnostics.result(Result.success(42L), Result.failure(cleanup)).exceptionOrNull())
        assertEquals(42L, AoaSecureDiagnostics.result(Result.success(42L), Result.success(Unit)).getOrThrow())
    }
    @Test fun t846DiagnosticsRetainBothFailurePhasesAndBoundCauseText() {
        val root = IOException("TLS failed", IllegalStateException("Native provider failed"))
        root.addSuppressed(IOException("TLS close also failed"))
        val cleanup = IOException("Native read did not retire")
        val result = AoaSecureDiagnostics.render(Result.failure(root), Result.failure(cleanup))
        assertTrue(result.contains("TLS failed")); assertTrue(result.contains("Native provider failed"))
        assertTrue(result.contains("TLS close also failed")); assertTrue(result.contains("Cleanup failed"))
        assertTrue(result.contains("Native read did not retire")); assertFalse(result.startsWith("PASS"))
        assertTrue(AoaSecureDiagnostics.render(Result.success(42L), Result.success(Unit)).startsWith("PASS"))
        val long = IOException("x".repeat(100000))
        val cycle = IOException("cycle")
        long.initCause(cycle); cycle.initCause(long)
        assertTrue(AoaSecureDiagnostics.describe(long).length < 4000)
    }

    @Test fun t846OwnedStackExcludesExternalFramesAndBoundsCycles() {
        val first = IOException("first")
        val second = IOException("second")
        first.initCause(second); second.initCause(first)
        first.stackTrace = arrayOf(StackTraceElement("external.Provider", "secret", "Provider.kt", 1),
            StackTraceElement("com.blent.Owned", "connect", "Owned.kt", 9))
        second.stackTrace = Array(1000) { StackTraceElement("com.blent.Other", "read", "Other.kt", it) }
        val text = AoaSecureDiagnostics.ownedStack(first)
        assertTrue(text.contains("com.blent.Owned.connect")); assertFalse(text.contains("external.Provider"))
        assertTrue(text.lines().size <= 72); assertTrue(text.length < 19000)
    }

}
