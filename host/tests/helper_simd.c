/* T778: environment admission happens before opening any capture resource. */
static void test_t778_simd_options(void) {
    assert(configure_conversion_simd(NULL) && g_conversion.simd == CONV_SIMD_AUTO);
    assert(configure_conversion_simd("portable") && g_conversion.simd == CONV_SIMD_PORTABLE);
    assert(configure_conversion_simd("auto") && g_conversion.simd == CONV_SIMD_AUTO);
    assert(configure_conversion_simd("avx2") == !!conv_simd_available(CONV_SIMD_AVX2));
    const char *invalid[] = {"", "junk", "AVX2", "avx2tail", "-1", "3", " portable"};
    for (unsigned i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++) {
        assert(!configure_conversion_simd(invalid[i]));
        assert(!setenv("BLENT_CONVERSION_SIMD", invalid[i], 1));
        char *argv[] = {"helper", "--edid", "/must-not-open", "--capture-fifo", "/must-not-open"};
        assert(evdi_helper_main(5, argv) == 2);
        assert(mock_discovery_calls == 0 && mock_add_calls == 0);
    }
    assert(!unsetenv("BLENT_CONVERSION_SIMD"));
}
