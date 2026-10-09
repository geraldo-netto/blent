package com.blent

/** Standard wire vocabulary; Native integer mappings belong to the platform adapter. */
internal object VideoProfiles {
    private val profiles = mapOf(
        "h264" to setOf("baseline" to 8, "constrained-baseline" to 8, "main" to 8, "high" to 8),
        "hevc" to setOf("main" to 8, "main10" to 10),
        "vp9" to setOf("profile0" to 8, "profile2" to 10),
        "av1" to setOf("main" to 8, "main" to 10),
    )
    val levels = mapOf(
        "h264" to listOf(10, 9, 11, 12, 13, 20, 21, 22, 30, 31, 32, 40, 41, 42, 50, 51, 52, 60, 61, 62),
        "hevc" to listOf(10, 20, 21, 30, 31, 40, 41, 50, 51, 52, 60, 61, 62),
        "vp9" to listOf(10, 11, 20, 21, 30, 31, 40, 41, 50, 51, 52, 60, 61, 62),
        "av1" to (2..7).flatMap { major -> (0..3).map { major * 10 + it } },
    )
    fun profile(codec: String, name: String, depth: Int): Boolean = profiles[codec]?.contains(name to depth) == true
    fun validLevel(codec: String, level: Int): Boolean = levels[codec]?.contains(level) == true
    fun identifier(value: String): Boolean = value.isNotEmpty() && value.length <= 128 && value.all { it.code in 33..126 }
}
