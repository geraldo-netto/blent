/* T778: forced runtime paths share the independent pixel/guard oracle. */
static void t778_oracle(conv_job_t job, unsigned char *expected, int edge) {
    job.ydst = expected; job.uvdst = expected + job.ow * job.oh;
    for (int cy = 0; cy < job.oh / 2; cy++) {
        for (int x = 0; x < job.ow; x += 2) {
            if (!edge || x < edge || x >= job.ow - edge) reference_block(&job, x, cy);
        }
    }
}

static void t778_wide(conv_pool_t *pool, int edge) {
    enum { W = 514, H = 8, STRIDE = W * 4 + 31, SIZE = W * H * 3 / 2, GUARD = 32 };
    unsigned char *storage = malloc(STRIDE * H + 1), actual[SIZE + GUARD], expected[SIZE + GUARD];
    assert(storage);
    unsigned char *source = storage + 1;
    for (int i = 0; i < STRIDE * H; i++) source[i] = (unsigned char)(i * 53 + i / 997);
    memset(actual, 0xA5, sizeof(actual)); memset(expected, 0xA5, sizeof(expected));
    pixel_row_t rows[H / 2];
    for (int cy = 0; cy < H / 2; cy++) rows[cy] = (pixel_row_t){.begin=0, .end=W, .gap={edge,W-edge}};
    conv_job_t job = {source, actual, actual + W * H, W, H, STRIDE, W, H, 1, 0, H/2, NULL, edge ? rows : NULL, PIXEL_BGRA};
    conv_pool_convert(pool, &job);
    t778_oracle(job, expected, edge);
    assert(!memcmp(actual, expected, sizeof(actual)) && "T778: runtime SIMD changes pixels or writes outside spans");
    t783_rgba_equivalent(pool, job, source, actual, expected, sizeof(actual));
    free(storage);
}

static void t778_policy(conv_pool_t *pool, conv_simd_t policy) {
    assert(conv_pool_set_simd(pool, policy));
    for (int scale = 1; scale <= 4; scale++)
        for (int pattern = 0; pattern < 5; pattern++) check_conversion(pool, scale, pattern);
    t778_wide(pool, 0); t778_wide(pool, 128); t778_wide(pool, 2);
}

static void t778_suite(void) {
    conv_pool_t pool = CONV_POOL_INITIALIZER;
    conv_pool_start(&pool, 4);
    const int invalid[] = {INT_MIN, -1, 3, 127, INT_MAX};
    for (unsigned i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++) {
        assert(!conv_pool_set_simd(&pool, (conv_simd_t)invalid[i]));
        assert(pool.simd == CONV_SIMD_AUTO);
    }
    t778_policy(&pool, CONV_SIMD_PORTABLE);
    conv_kernel_t portable = pool.kernel;
    t778_policy(&pool, CONV_SIMD_AUTO);
    if (conv_simd_available(CONV_SIMD_AVX2)) {
        t778_policy(&pool, CONV_SIMD_AVX2);
        assert(pool.kernel != portable && "T778: forcing AVX2 must select the accelerated kernel");
    }
    else assert(!conv_pool_set_simd(&pool, CONV_SIMD_AVX2));
    assert(!conv_simd_available((conv_simd_t)INT_MAX));
    for (unsigned i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++) {
        conv_job_t invalid_job = {.order=(pixel_order_t)invalid[i]};
        conv_pool_convert(&pool, &invalid_job);
        assert(pool.last_jobs == 0 && "T783: invalid pixel order must reject before touching null buffers");
    }
    conv_pool_destroy(&pool);
}
