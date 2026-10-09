/* T782: box averaging keeps per-pixel truncation before chroma averaging. */
static void t782_case(conv_pool_t *pool, int scale, int width, int edge) {
    enum { H = 8, GUARD = 32 };
    int sw = width * scale + 1, sh = H * scale + 1, stride = sw * 4 + 13;
    size_t size = (size_t)width * H * 3 / 2;
    unsigned char *storage = malloc((size_t)stride * sh + 1);
    unsigned char *actual = malloc(size + GUARD), *expected = malloc(size + GUARD);
    assert(storage && actual && expected);
    unsigned char *source = storage + 1; unsigned seed = 782;
    for (int i = 0; i < stride * sh; i++) {
        seed = seed * 1664525u + 1013904223u; source[i] = (unsigned char)(seed >> 24);
    }
    memset(actual, 0xA5, size + GUARD); memset(expected, 0xA5, size + GUARD);
    pixel_row_t rows[H / 2];
    for (int cy = 0; cy < H / 2; cy++) rows[cy] = (pixel_row_t){.begin=0, .end=width, .gap={edge,width-edge}};
    conv_job_t job = {source, actual, actual + width * H, sw, sh, stride, width, H, scale, 0, H/2, NULL, edge ? rows : NULL, PIXEL_BGRA};
    conv_pool_convert(pool, &job);
    t778_oracle(job, expected, edge);
    assert(!memcmp(actual, expected, size + GUARD) && "T782: box reduction changes rounding, gap pixels or guards");
    t783_rgba_equivalent(pool, job, source, actual, expected, size + GUARD);
    free(storage); free(actual); free(expected);
}

static void t782_policy(conv_pool_t *pool, conv_simd_t policy) {
    assert(conv_pool_set_simd(pool, policy));
    const int widths[] = {2, 6, 14, 30, 62, 126, 128, 130, 254, 258, 514};
    for (int scale = 2; scale <= 4; scale++) {
        for (unsigned i = 0; i < sizeof(widths) / sizeof(widths[0]); i++) t782_case(pool, scale, widths[i], 0);
        t782_case(pool, scale, 514, 128); t782_case(pool, scale, 514, 2);
    }
}

static void t782_suite(void) {
    conv_pool_t pool = CONV_POOL_INITIALIZER; conv_pool_start(&pool, 4);
    t782_policy(&pool, CONV_SIMD_PORTABLE); t782_policy(&pool, CONV_SIMD_AUTO);
    if (conv_simd_available(CONV_SIMD_AVX2)) t782_policy(&pool, CONV_SIMD_AVX2);
    conv_pool_destroy(&pool);
}
