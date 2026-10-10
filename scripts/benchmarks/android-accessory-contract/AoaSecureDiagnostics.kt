package com.blent

/** Profile qualification outcome: transfer and cleanup are separate evidence. */
internal object AoaSecureDiagnostics {
    fun result(exchanged: Result<Long>, retired: Result<Unit>): Result<Long> {
        if (exchanged.isFailure) return exchanged
        return if (retired.isFailure) Result.failure(retired.exceptionOrNull()!!) else exchanged
    }
    fun describe(error: Throwable): String {
        val causes = generateSequence(error) { it.cause }.take(6).map(::brief).joinToString("; caused by ")
        val suppressed = error.suppressed.take(4).joinToString("; ", transform = ::brief)
        return if (suppressed.isEmpty()) causes else "$causes; suppressed: $suppressed"
    }
    fun ownedStack(error: Throwable): String = generateSequence(error) { it.cause }.take(6)
        .flatMap { cause -> cause.stackTrace.asSequence().filter { it.className.startsWith("com.blent.") }.take(12) }
        .joinToString("\n") { it.toString().take(256) }

    private fun brief(error: Throwable): String =
        "${error.javaClass.simpleName}: ${error.message.orEmpty().take(512).replace('\n', ' ').replace('\r', ' ')}"

    fun render(exchanged: Result<Long>, retired: Result<Unit>): String {
        val result = result(exchanged, retired).fold(
            { "PASS: 4096 encrypted patterned frames; $it bytes echoed across six channels; host receipt and bridge retirement confirmed." },
            { "Encrypted probe ended: ${describe(it)}" },
        )
        val cleanup = retired.exceptionOrNull() ?: return result
        return "$result\nCleanup failed: ${describe(cleanup)}"
    }
}
