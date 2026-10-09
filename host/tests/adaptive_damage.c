/* T768: distant updates must skip the unchanged gap, without tile rounding. */
static void t768_case(int scale, int fragmented) {
    enum { W = 512, H = 8, SIZE = W * H * 3 / 2, GUARD = 32 };
    int sw = W * scale + 1, sh = H * scale + 1, stride = sw * 4 + 64;
    unsigned char *source = calloc(stride, sh), expected[SIZE + GUARD];
    assert(source);
    frame_exchange_t f = FRAME_EXCHANGE_INITIALIZER;
    frame_exchange_resize(&f, W, H); assert(frame_exchange_allocated(&f));
    memset(f.dirty_fill, 0, f.dirty_bytes);
    memset(f.fill, 0xA5, f.size); memset(expected, 0xA5, sizeof(expected));
    frame_exchange_damage_rect(&f, 0, 0, 16 * scale, H * scale, scale);
    frame_exchange_damage_rect(&f, (W - 16) * scale, 0, W * scale, H * scale, scale);
    if (fragmented) {
        for (int x = 32; x < W - 16; x += 32)
            frame_exchange_damage_rect(&f, x * scale, 0, (x + 2) * scale, H * scale, scale);
    }
    conv_pool_t pool = CONV_POOL_INITIALIZER;
    conv_job_t job = {source, f.fill, f.fill + W * H, sw, sh, stride, W, H,
        scale, 0, H / 2, f.dirty_fill, f.spans_fill, PIXEL_BGRA};
    conv_pool_convert(&pool, &job);
    job.ydst = expected; job.uvdst = expected + W * H;
    for (int cy = 0; cy < H / 2; cy++) {
        for (int x = 0; x < W; x += 2) {
            if (fragmented || x < 16 || x >= W - 16) reference_block(&job, x, cy);
        }
    }
    assert(!memcmp(f.fill, expected, SIZE) && "T768: adaptive conversion must skip large gaps and coalesce fragmented work");
    frame_exchange_free(&f); pthread_mutex_destroy(&f.mutex); conv_pool_destroy(&pool); free(source);
}

static void t768_union_covered(const frame_exchange_t *f, const unsigned char *expected) {
    pixel_row_t row = f->spans_fill[0];
    for (int x = 0; x < f->width; x += 2) {
        if (!expected[x / 2]) continue;
        assert(x >= row.begin && x < row.end);
        assert(x < row.gap.begin || x >= row.gap.end);
    }
    if (row.gap.begin < row.gap.end)
        assert(row.gap.begin >= row.begin && row.gap.end <= row.end);
}

static void t768_union(void) {
    frame_exchange_t f = FRAME_EXCHANGE_INITIALIZER;
    frame_exchange_resize(&f, 512, 2); assert(frame_exchange_allocated(&f));
    unsigned char expected[256]; unsigned state = 768;
    for (int trial = 0; trial < 256; trial++) {
        memset(f.dirty_fill, 0, f.dirty_bytes); memset(expected, 0, sizeof(expected));
        for (int rectangle = 0; rectangle < 8; rectangle++) {
            state = state * 1664525u + 1013904223u;
            int a = (int)(state % 256), b = (a + 1 + (int)((state >> 8) % 64));
            if (b > 256) b = 256;
            frame_exchange_damage_rect(&f, 2 * a, 0, 2 * b, 2, 1);
            memset(expected + a, 1, b - a);
            t768_union_covered(&f, expected);
        }
    }
    frame_exchange_free(&f); pthread_mutex_destroy(&f.mutex);
}

static void t768_threaded_oracle(conv_job_t job, unsigned char *expected) {
    memset(expected, 0xA5, job.ow * job.oh * 3 / 2);
    job.ydst = expected; job.uvdst = expected + job.ow * job.oh;
    for (int cy = 0; cy < job.oh / 2; cy++) {
        for (int x = 0; x < 128; x += 2) {
            reference_block(&job, x, cy);
            reference_block(&job, job.ow - 128 + x, cy);
        }
    }
}

static void t768_threaded(void) {
    enum { W=1024, H=128, SCALE=4, SIZE=W*H*3/2 };
    unsigned char *source=calloc(W*SCALE*4,H*SCALE), *actual=malloc(SIZE), *expected=malloc(SIZE);
    assert(source && actual && expected);
    pixel_row_t rows[H/2]; unsigned char dirty[H/16]; memset(dirty,0xFF,sizeof(dirty));
    for (int cy=0;cy<H/2;cy++) rows[cy]=(pixel_row_t){.begin=0,.end=W,.gap={128,W-128}};
    conv_job_t job={source,actual,actual+W*H,W*SCALE,H*SCALE,W*SCALE*4,W,H,SCALE,0,H/2,dirty,rows, PIXEL_BGRA};
    conv_pool_t pool=CONV_POOL_INITIALIZER; conv_pool_start(&pool,4);
    t768_threaded_oracle(job, expected);
    for (int use_mask=0;use_mask<2;use_mask++) {
        memset(actual,0xA5,SIZE); job.dirty=use_mask ? dirty : NULL;
        conv_pool_convert(&pool,&job);
        assert(pool.last_jobs==(use_mask ? 2 : 4));
        assert(!memcmp(actual,expected,SIZE) && "T768: threaded gaps must match the independent oracle");
    }
    conv_pool_destroy(&pool); free(source); free(actual); free(expected);
}

static void t768_suite(void) {
    for (int scale = 1; scale <= 4; scale++) {
        t768_case(scale, 0); t768_case(scale, 1);
    }
    t768_union(); t768_threaded();
}
