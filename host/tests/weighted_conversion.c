/* T769: independent pixel oracle plus bounded scheduling-work imbalance. */
static size_t t769_width(const conv_job_t *j, int row) {
    if (!(j->dirty[row / 8] & (1u << (row % 8)))) return 0;
    return (size_t)(j->spans[row].end - j->spans[row].begin);
}

static size_t t769_allowance(const conv_job_t *job, int jobs) {
    size_t total = 0, biggest_row = 0;
    for (int cy = job->cy0; cy < job->cy1; cy++) {
        size_t width = t769_width(job, cy);
        total += width;
        if (width > biggest_row) biggest_row = width;
    }
    return (total + jobs - 1) / jobs + biggest_row;
}

static void t769_partitions(const conv_pool_t *pool, const conv_job_t *job) {
    if (pool->last_jobs < 2) return;
    size_t allowance = t769_allowance(job, pool->last_jobs);
    int begin = job->cy0;
    for (int i = 0; i < pool->last_jobs; i++) {
        const conv_job_t *part = &pool->jobs[i];
        assert(part->cy0 == begin && part->cy1 >= begin && part->cy1 <= job->cy1);
        size_t width = 0;
        for (int cy = part->cy0; cy < part->cy1; cy++) width += t769_width(job, cy);
        assert(width <= allowance
               && "T769: row-count partition overloads workers with wide rows");
        begin = part->cy1;
    }
    assert(begin == job->cy1 && "T769: every row belongs to exactly one job");
}

static void t769_pattern(conv_job_t *job, unsigned char *dirty, pixel_row_t *spans, int pattern) {
    memset(dirty, 0, (size_t)(job->cy1 + 7) / 8);
    for (int cy = 0; cy < job->cy1; cy++) {
        spans[cy] = (pixel_row_t){.begin=0, .end=cy < job->cy1 / 2 ? job->ow : 32};
        if (pattern == 1 && cy % 3) continue;
        if (pattern == 2 && cy != job->cy1 / 2) continue;
        dirty[cy / 8] |= (unsigned char)(1u << (cy % 8));
    }
}

static void t769_case(conv_pool_t *pool, int scale, int pattern) {
    enum { W = 1920, H = 1088, STRIDE = W * 4 + 64, GUARD = 32 };
    int ow = (W / scale) & ~1, oh = (H / scale) & ~1, size = ow * oh * 3 / 2;
    unsigned char *source = malloc((size_t)STRIDE * H), *actual = malloc(size + GUARD), *expected = malloc(size + GUARD);
    assert(source && actual && expected);
    for (size_t n = 0; n < (size_t)STRIDE * H; n++) source[n] = (unsigned char)(n * 37 + n / 997);
    memset(actual, 0xA5, size + GUARD); memset(expected, 0xA5, size + GUARD);
    unsigned char dirty[(H / 2 + 7) / 8]; pixel_row_t spans[H / 2];
    conv_job_t job = {source, actual, actual + ow * oh, W, H, STRIDE, ow, oh,
        scale, 0, oh / 2, dirty, spans, PIXEL_BGRA};
    t769_pattern(&job, dirty, spans, pattern);
    conv_pool_convert(pool, &job);
    assert(pool->last_jobs <= pool->count);
    t769_partitions(pool, &job);
    job.ydst = expected; job.uvdst = expected + ow * oh;
    for (int cy = 0; cy < job.cy1; cy++) {
        if (!t769_width(&job, cy)) continue;
        for (int x = spans[cy].begin; x < spans[cy].end; x += 2) reference_block(&job, x, cy);
    }
    assert(!memcmp(actual, expected, size + GUARD) && "T769: weighted dispatch must preserve pixels and guards");
    free(source); free(actual); free(expected);
}

static void t769_suite(void) {
    const int capacities[] = {1, 4, 8};
    for (unsigned i = 0; i < sizeof(capacities) / sizeof(capacities[0]); i++) {
        conv_pool_t pool = CONV_POOL_INITIALIZER;
        conv_pool_start(&pool, capacities[i]);
        for (int scale = 1; scale <= 4; scale++)
            for (int pattern = 0; pattern < 3; pattern++) t769_case(&pool, scale, pattern);
        conv_pool_destroy(&pool);
    }
}
