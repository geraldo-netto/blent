/* T779: overlap costs at most one full swizzle; clipped pixels stay exact. */
static void t779_paint(struct evdi_rect rect) {
    for (int y = 0; y < 6; y++) {
        for (int x = 0; x < 6; x++) {
            if (!t752_contains(rect.x1, rect.x2, x) || !t752_contains(rect.y1, rect.y2, y)) continue;
            unsigned char *p = g_capture.native_framebuffer + y * g_capture.mode_stride + 4 * x;
            p[0] = x * 31; p[1] = y * 37; p[2] = 255 - x * 17; p[3] = 123;
        }
    }
}

static void t779_check_pixels(void) {
    for (int y = 0; y < 6; y++) {
        for (int x = 0; x < 6; x++) {
            const unsigned char *src = g_capture.native_framebuffer + y * g_capture.mode_stride + 4 * x;
            const unsigned char *dst = g_capture.framebuffer + y * g_capture.mode_stride + 4 * x;
            assert(dst[0] == src[2] && dst[1] == src[1]);
            assert(dst[2] == src[0] && dst[3] == src[3]);
        }
        for (int x = 24; x < g_capture.mode_stride; x++)
            assert(g_capture.framebuffer[y * g_capture.mode_stride + x] == 0x18);
    }
}

static void t779_case(const struct evdi_rect *rects, int count) {
    memset(g_capture.native_framebuffer, 0x18, g_capture.fb_size);
    memset(g_capture.framebuffer, 0x18, g_capture.fb_size);
    for (int i = 0; i < count; i++) t779_paint(rects[i]);
    t779_normalized_pixels = 0;
    normalize_framebuffer(&g_capture, rects, count);
    assert(t779_normalized_pixels <= 36 && "T779: repeated rectangles exceed full-frame work");
    t779_check_pixels();
}

static void test_t779_normalization_work(void) {
    frame_exchange_init(&g_frames); g_running = 1;
    on_mode_changed((struct evdi_mode){6,6,60,32,0x34324258}, &g_capture);
    assert(g_capture.have_mode);
    struct evdi_rect rects[63];
    for (int i = 0; i < 63; i++) rects[i] = (struct evdi_rect){0,0,6,6};
    t779_case(rects, 63);
    const int bounds[] = {INT_MIN, -7, -1, 0, 1, 3, 6, 7, INT_MAX};
    unsigned state = 779;
    for (int trial = 0; trial < 256; trial++) {
        int count = 1 + trial % 63;
        for (int i = 0; i < count; i++) {
            state = state * 1664525u + 1013904223u;
            rects[i] = (struct evdi_rect){bounds[state % 9], bounds[(state >> 8) % 9],
                bounds[(state >> 16) % 9], bounds[(state >> 24) % 9]};
        }
        t779_case(rects, count);
    }
    free(g_capture.framebuffer); free(g_capture.native_framebuffer);
    frame_exchange_free(&g_frames); pthread_cond_destroy(&g_frames.ready);
}
